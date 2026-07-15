# Ovsy | AI Assistant Kernel

![Cover](/assets/cover.png)

## Overview

**Ovsy** is a high-performance **AI Assistant Kernel** designed around a modular, multi-agent architecture.
Unlike generic AI wrappers, Ovsy operates as a tightly integrated orchestrator that delegates tasks
to specialized background agents.<br>

To achieve bare-metal execution speed and strict resource control, Ovsy bypasses generalized industry standards
like the **Model Context Protocol (MCP)** in favor of a custom, streamlined **Server-Sent Events (SSE)** protocol.<br>

This document outlines the structural design of the Ovsy assistant, its core orchestration loop,
and the technical rationale behind its communication architecture.

## Orchestration Architecture

![Scheme](/assets/scheme.png)

The Ovsy assistant coordinates user queries through a multi-stage orchestration engine governed
by a central execution loop (**handle_task**).

> `User Query` ➔ `Orchestrator Evaluation` ➔ `Concurrent Task Spawning` ➔ `Self-Healing Loop / Resolution`

1. **Task Evaluation and Execution**

When a user submits a query, the orchestrator determines if background tasks are required:

* **Parallel Execution:** The engine builds a dependency graph of the required actions.
  Tasks without active upstream dependencies bypass sequential queues and are instantly spawned across separate threads.
  This allows multiple agents or tools to execute concurrently during a single generation cycle.

* **Context Isolation:** To prevent token bloat and reduce latency, agents do not receive the entire conversation history.
  Instead, context is strictly limited to the specific task description and the exact output payloads of any dependent upstream tasks.
  Every agent maintains its own isolated system prompt and configuration.

2. **Modularity via Skills & Context Optimization**

To prevent massive token bloat and reduce model cognitive load, Ovsy replaces monolithic agent definitions with a dynamic Skill-based architecture.

* **Granular Tool Grouping:** Agents no longer expose a flat, undivided list of tools to the orchestrator.
  Instead, capabilities are logically grouped into Skills (e.g., SystemInfo, AudioControl, MusicPlayback, PowerManagement).
* **Dynamic Loading (/tools/list):** When preparing to execute an AgentTask, the orchestrator checks the required agent_skills array.
  Instead of injecting the agent's entire toolset into the LLM context, the kernel issues a filtered request to the agent’s /tools/list endpoint,
  fetching only the tool definitions corresponding to the active skills required for the task.
* **Metadata-Only Cache:** The kernel's ensure_agent routine operates on lightweight AgentMetadata rather than full tool schemas,
  drastically lowering idle memory usage and improving startup time.

3. **Deep Self-Healing Loop**

The orchestrator embeds a robust Self-Healing Loop to recover from model failures, transmission anomalies, or execution exceptions in real time.

* **Empty Output & Hallucination Recovery:** If the LLM generates an empty text response or fails to call a necessary tool when expected, the loop intercepts the anomaly.
  The orchestrator automatically appends a corrective prompt (e.g., "You returned an empty response.
  If you need to resolve this, delegate tasks to the appropriate agents...") and triggers a retry.
* **Execution & Parsing Resiliency:** If a downstream tool execution fails, or if the agent returns corrupted JSON arguments during a Tool/JS execution,
  the error is captured, converted into system context, and fed back into the next generation attempt.
* **Response Synthesis:** Upon successful tool execution, rather than directly outputting raw data payloads,
  the orchestrator routes the aggregated tool outputs through a final LLM synthesis pass, generating a clean, human-friendly response.
* **Strict Loop Bounds:**
    * **Configurable Retry Cap:** The recursion is strictly bounded by max_retries defined in the CLI configurations (AssistantOptions),
      replacing the older flat loop bounds (max_cycles).
    * **Critical Failure Bypass:** Fatal infrastructure exceptions (such as network failure or database lockouts) immediately break the execution loop,
      avoiding redundant calls and protecting computing resources.

4. **Embedded JavaScript Interpreter**

To eliminate unnecessary orchestration cycles, Ovsy embeds the **Boa Engine** JavaScript interpreter directly into the execution pipeline.

* **Inline Expression Evaluation:** Agents can execute arbitrary JavaScript snippets without spawning an external runtime,
  allowing lightweight computations, data transformations, and conditional logic to be performed with minimal overhead.
* **Direct Task Routing:** Instead of returning the evaluation result to the orchestrator and triggering a new planning iteration,
  the interpreter can immediately forward the produced value to another task by its `task_id`.
  This enables dependent tasks to continue execution within the same orchestration cycle, reducing latency and avoiding redundant LLM generations.
* **Reduced Token Consumption:** Since intermediate results no longer need to be serialized into the conversation and reinterpreted during the next planning pass,
  the assistant significantly decreases token usage while improving overall throughput.

## Process Lifecycle and Safety

Ovsy relies on low-level operating system process hierarchies to enforce architectural stability,
maintain isolated crash boundaries, and eliminate resource leaks.
The `Agent` component encapsulates the initialization, orchestration, and teardown of these external subprocesses.

### Process Isolation

To guarantee that an individual agent failure cannot compromise the host environment,
the engine implements strict runtime boundaries and three layers of defensive termination:

* **Zero Zombie Processes:** Agents are registered as native child processes directly linked to the core orchestrator kernel.
  If the main server terminates unexpectedly or is killed, the operating system kernel automatically reaps all child processes,
  ensuring no orphaned processes remain active in memory.

* **Isolated Crash Boundaries:** Fault tolerance is strictly isolated. If an individual agent experiences a fatal crash,
  it does not impact the main orchestrator or sibling agents. The assistant logs the failure to disk,
  flags the impacted pipeline branch, and continues running smoothly, laying the groundwork
  for automated agent hot-restarts in upcoming updates.

* **RAII-Based Cleanup:** The underlying command is configured with `kill_on_drop(true)`.
  This ensures that when the `Agent` struct instance drops out of memory scope,
  the corresponding OS subprocess is automatically signaled to terminate.

* **Linux-Specific Protection:** On Linux environments, the engine invokes a safe abstraction over `libc::prctl` using
  the `PR_SET_PDEATHSIG` flag coupled with `SIGKILL`. This instructs the kernel to immediately kill the agent process
  if the main orchestrator application crashes or exits unexpectedly.

* **macOS-Specific Protection:** On macOS, due to the absence of native process death signals like Linux's `PR_SET_PDEATHSIG`,
  the engine spawns a dedicated background thread that monitors the `stdin` (standard input) stream. Since the orchestrator
  holds the writing end of the `pipe`, any unexpected termination of the main server (including `kill -9`) instantly triggers
  an `End-of-File (EOF)` on the agent's side, allowing the thread to immediately intercept the event and exit the process gracefully.

* **Windows Job Objects:** On Windows, processes are instantiated via group spawning (`spawn_group`).
  This encapsulates the agent inside a Windows Job Object, ensuring clean, cascaded process tree termination.

### Network Isolation

To minimize context-switching overhead and maximize data throughput, agent communication is completely decoupled from the network stack.
Instead of utilizing traditional TCP loopback interfaces, the orchestrator and agents communicate via local **Unix Domain Sockets (UDS)** (`AF_UNIX` on Windows).

* **File-System-Level Security:** Each agent process binds to a unique, dynamically generated socket file (`/tmp/ovsy/uds/{agent_name}.sock`).
  Access control is strictly enforced using standard POSIX file permissions, completely isolating agent endpoints from external network ingress or unauthorized local processes.

* **Zero Network Overhead:** By leveraging UDS, the system bypasses the entire routing, filtering, and TCP/IP loopback overhead of the operating system kernel.
  This enables zero-copy-like data streaming directly between the orchestrator's async runtime and the agent processes via specialized IPC pipes.

* **Dynamic Descriptors:** Sockets are provisioned and lifecycle-managed automatically by the manager layer. When an agent drops out of scope or is killed,
  its corresponding socket descriptor on the filesystem is cleanly unlinked and destroyed to prevent resource leakage.

### Initialization

The system utilizes a split-phase verification strategy to guarantee that an agent is fully prepared to accept workloads before it is exposed to the supervisor.

* **Startup Polling Loop:** Upon process spawning, the manager enters a strict polling loop targeting the agent's /init HTTP endpoint.
  To accommodate process spin-up times, heavy binary initialization, or cold starts, the loop allows up to 50 attempts spaced out by 100ms intervals,
  granting the agent a reliable 5-second window to warm up.

* **Deadlock Prevention:** The HTTP handshake is wrapped in a tight `tokio::time::timeout` guard (100ms per request).
  This critical safety boundary prevents the orchestrator from blocking indefinitely if the agent successfully binds
  to the TCP port but hangs internally during its boot sequence.

* **Liveness Monitoring:** Subsequent health monitoring via the `check` method periodically verifies the agent's state by attempting a shallow connection to its Unix Domain Socket.
  If the connection is refused, times out, or the socket file disappears from the filesystem, the agent is immediately marked as dead,
  triggering automated recovery or hot-reloading routines.

* **Hot-Reloading:** The `check` method continuously compares the on-disk binary metadata timestamp
  against the process instantiation time (`_started`). If the binary has been overwritten or updated,
  the orchestrator flags the agent for a graceful restart.

## Communication Protocol

Agents within the **Ovsy** kernel function as independent, long-running servers that initialize once at startup.
Rather than repeatedly spawning and destroying processes, they remain active in memory. If a configuration changes,
an explicit control command (`ovsy update`) reloads only the modified targets without taking down the broader application network.

Communication between the orchestrator and agents relies on a custom SSE protocol utilizing specialized events (`Event`).
The data payload is structured around a lightweight layout:
```rust
pub enum EventKind {
    Start,
    Thinking,
    Answer,
    Error,
    Finish,
}

pub struct EventTaskInfo {
    pub task_id: i64,
    pub tool_call_id: String,
}

pub struct Event {
    pub kind: EventKind,
    pub task_info: Option<EventTaskInfo>,
    pub text: String,
}
```

### Server Endpoints

To manage the server, coordinate AI agents, and optimize dialogue context,
the orchestrator provides a set of dedicated endpoints.

> All requests are made using the POST method, and both request and response bodies expect JSON format.

  * **POST `/status`** Returns the current state of the server along with a list of all currently running agents
    (including their IDs, statuses, and workload).

  * **POST `/refresh`**: Dynamically applies server configuration changes from the config file.
    If necessary, it restarts outdated agents and initializes newly added,
    non-running agents on the fly without a full server reboot.

  * **POST `/users/{uid}/sessions`**: Retrieves a list of all existing user sessions by their ID.
    * **limit (optional)**: Specifies the number of the most recent sessions to return.

  * **POST `/sessions/{sid}/get`**: Retrieves the message history for a specific session by its `SessionID`.
    Used to restore the conversation context in the UI.

  * **POST `/sessions/{sid}/clear`**: Clears the session history by `SessionID`, deleting all associated messages
    from the database. Allows resetting the context to start a fresh conversation.

  * **POST `/sessions/{sid}/query`**: The primary endpoint for interacting with the system.
    It receives a full user dialog (messages), routes it to the appropriate agents,
    and returns the generated response as SSE.
    * **message**: The new user message to be processed and appended to the conversation history.
  
  * **POST `/sessions/{sid}/compact`**: Utilizes AI to summarize (compress) the chat history.
    This helps reduce token consumption while preserving the essential context of the conversation.
    * **preserve (optional)**: The number of recent message pairs (User -> Assistant + Tool) to keep intact (uncompressed)
    at the end of the history.
 
### Agent Endpoints

To keep routing overhead at an absolute minimum, each agent exposes an intentionally minimalist API surface.
The endpoints are designed to handle initialization, tracking, and execution with zero computational waste.

  * **POST `/ping`:** Used for real-time telemetry, health diagnostics, and log tracing.
    Instead of a simple uptime signal, it returns structural metadata allowing the orchestrator
    to instantly locate the agent's operational footprint.

  * **POST `/init`:** Exposes the agent’s profile, system prompt, and capabilities.
    This endpoint is invoked exclusively during the agent's initial bootstrap when the assistant starts,
    as well as during hot-reloads via the ovsy update command.

  * **POST `/tools/list`:** Returns the catalog of tools exposed by the agent, along with their metadata, parameters, and associated skills.
    Supports filtering by sending a payload of specific skills to retrieve only the subset of tools relevant to a particular task domain.

  * **POST `/tools/call/{tool}`:** Handles direct, isolated execution of a specific tool requested by the orchestrator.
    By binding each execution to a dedicated, strict context, it minimizes the risk of model hallucinations
    and ensures that the tool's payload is utilized to its maximum efficiency.

### Architectural Rationale

The Model Context Protocol (MCP) has gained traction as a generic standard for linking models to external tools.
However, for a production-grade, high-concurrency AI assistant like Ovsy,
universal standards introduce severe engineering compromises:

  * **System Call Overhead:** MCP relies heavily on standardized **JSON-RPC** wrappers and multiple layers
  of **Inter-Process Communication (IPC)**. This abstraction layer forces an excessive volume of system calls,
  creating latency bottlenecks under heavy multi-agent workloads.

  * **Memory Bloat:** Because MCP is designed to accommodate any arbitrary system,
  it requires packaging and parsing bloated, generalized context objects.
  This approach fundamentally violates zero-copy principles and leads to unnecessary **RAM** consumption.

  * **Control and Abstraction:** By utilizing a tailored, SSE-over-UDS eventing design, Ovsy eliminates the abstraction tax of generalized protocols.
  The data pipeline is stripped down to native Unix domain communication, passing isolated streaming payloads directly to the Tokio execution threads
  with sub-millisecond coordination latency.

## Framework Stack

To maintain a clean and highly maintainable core codebase, Ovsy delegates complex lower-level operations to dedicated,
decoupled library frameworks:

| Framework   | Core Responsibility                                                                         | Technical Base           |
| :---        | :---                                                                                        | :---                     |
| **Atoman**  | Asynchronous feature management and memory-safe data ownership across threads.              | Custom Concurrency Layer |
| **AnyLM**   | Universal AI SDK abstracting cloud APIs and local inference engines into a single protocol. | Standardized SDK         |
| **Pearce**  | Ultra-lean, high-throughput TCP web server framework powering agent communication.          | Built on Axum            |
| **Cistern** | High-performance, asynchronous Retrieval-Augmented Generation & Key-Value database engine.  | Built on LanceDB & Sled  |

> By keeping the core architecture slim and offloading specialized tasks to this robust underlying stack,
Ovsy offers developers and stakeholders a clean, enterprise-ready AI assistant environment optimized for speed,
predictability, and structural safety.

## License & Feedback

> This software distributed under the [GPL 3.0](https://github.com/fuderis/ovsy/blob/main/LICENSE.md) license.

You can contact me via [GitHub](https://github.com/fuderis) or send a message to my [E-Mail](mailto:synapdrake@ya.ru).
This library is actively evolving, and your suggestions and feedback are always welcome!
