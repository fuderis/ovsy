# Ovsy Assistant

## Overview

**Ovsy** is a high-performance **AI Assistant Kernel** designed around a modular, multi-agent architecture.
Unlike generic AI wrappers, Ovsy operates as a tightly integrated orchestrator that delegates tasks
to specialized background agents.<br>

To achieve bare-metal execution speed and strict resource control, Ovsy bypasses generalized industry standards
like the **Model Context Protocol (MCP)** in favor of a custom, streamlined **Server-Sent Events (SSE)** protocol.<br>

This document outlines the structural design of the Ovsy assistant, its core orchestration loop,
and the technical rationale behind its communication architecture.

## Orchestration Architecture

The Ovsy assistant coordinates user queries through a multi-stage orchestration engine governed
by a central execution loop (**handle_task**).

> `User Query` ➔ `Orchestrator Evaluation` ➔ `Concurrent Task Spawning` ➔ `Self-Healing Loop / Resolution`

1. **Task Evaluation and Execution**

When a user submits a query, the orchestrator determines if background tasks are required:
  * **Parallel Execution:** The engine builds a dependency graph of the required actions. Tasks without active upstream dependencies bypass sequential queues and are instantly spawned across separate threads. This allows multiple agents or tools to execute concurrently during a single generation cycle.
  * **Context Isolation:** To prevent token bloat and reduce latency, agents do not receive the entire conversation history. Instead, context is strictly limited to the specific task description and the exact output payloads of any dependent upstream tasks. Every agent maintains its own isolated system prompt and configuration (AgentInfo).

2. **Embedded Self-Healing Loop**

If a running agent encounters an error during execution, the orchestrator initiates a recursive cycle to attempt an automatic fix.
  * **Configurable Iteration Cap:** To prevent execution traps, the maximum number of recursive
    self-healing attempts is strictly bounded by a limit defined in the system configuration file (`config/settings.toml`).
  * **Critical Failure Bypass:** For critical infrastructure errors—such as a complete network failure
    when connecting to an external LLM provider—the engine immediately halts the loop, bypassing recursion entirely
    to conserve computing resources.
  * **Automated Diagnostics:** All runtime exceptions, operational errors, and critical failures are automatically flushed
    to disk via the system logger (`logs/errors`) to ensure full offline traceability for developers.

## Communication Protocol

Agents within the **Ovsy** kernel function as independent, long-running servers that initialize once at startup.
Rather than repeatedly spawning and destroying processes, they remain active in memory. If a configuration changes,
an explicit control command (`ovsy update`) reloads only the modified targets without taking down the broader application network.

Communication between the orchestrator and agents relies on a custom SSE protocol utilizing specialized data chunks (`Chunk`).
The data payload is structured around a lightweight enum layout:
```rust
struct Chunk {
    agent: Option<AgentTask>,
    data: ChunkData,
}

enum ChunkData {
    Tools(Vec<ToolCall>),
    Thinking(String),
    Answer(String),
    Error(String),
    Finish,
}
```

## Server Endpoints

To manage the server, coordinate AI agents, and optimize dialogue context,
the orchestrator provides a set of dedicated endpoints.

> All requests are made using the POST method, and both request and response bodies expect JSON format.

  * **POST `/handle`**: The primary endpoint for interacting with the system.
  It receives a full user dialog (messages), routes it to the appropriate agents,
  and returns the generated response as SSE.
  
  * **POST `/compact`**: Utilizes AI to summarize (compress) the chat history.
  This helps reduce token consumption while preserving the essential context of the conversation.

  * **POST `/status`** Returns the current state of the server along with a list of all currently running agents
    (including their IDs, statuses, and workload).

  * **POST `/update`**: Dynamically applies server configuration changes from the config file.
  If necessary, it restarts outdated agents and initializes newly added,
  non-running agents on the fly without a full server reboot.

## Agent Endpoints

To keep routing overhead at an absolute minimum, each agent exposes an intentionally minimalist API surface.
The endpoints are designed to handle initialization, tracking, and execution with zero computational waste.

  * **POST `/ping`:** Used for real-time telemetry, health diagnostics, and log tracing.
  Instead of a simple uptime signal, it returns structural metadata allowing the orchestrator
  to instantly locate the agent's operational footprint.

  * **POST `/info`:** Exposes the agent’s profile, system prompt, and capabilities.
  This endpoint is invoked exclusively during the agent's initial bootstrap when the assistant starts,
  as well as during hot-reloads via the `ovsy update` command.

  * **POST `/call/{tool}`:** Handles direct, isolated execution of a specific tool requested by the orchestrator.
  By binding each execution to a dedicated, strict context, it minimizes the risk of model hallucinations
  and ensures that the tool's payload is utilized to its maximum efficiency.

## Architectural Rationale

The Model Context Protocol (MCP) has gained traction as a generic standard for linking models to external tools.
However, for a production-grade, high-concurrency AI assistant like Ovsy,
universal standards introduce severe engineering compromises:

  * **System Call Overhead:** MCP relies heavily on standardized **JSON-RPC** wrappers and multiple layers
    of **Inter-Process Communication (IPC)**. This abstraction layer forces an excessive volume of system calls,
    creating latency bottlenecks under heavy multi-agent workloads.
  * **Memory Bloat:** Because MCP is designed to accommodate any arbitrary system,
    it requires packaging and parsing bloated, generalized context objects.
    This approach fundamentally violates zero-copy principles and leads to unnecessary **RAM** consumption.
  * **Control and Abstraction:** By utilizing a tailored, SSE-based chunking design, Ovsy eliminates the abstraction
    tax of generalized protocols.<br>

    The data pipeline is built exclusively for what the assistant needs:
    streaming reasoning steps, handling targeted tool requests, and passing isolated outputs directly to the threads that need them.

## Process Lifecycle

Ovsy relies on low-level operating system process hierarchies to enforce stability and eliminate resource leaks.
  * **Zero Zombie Processes:** Agents are registered as native child processes directly linked to the core orchestrator kernel.
    If the main server terminates unexpectedly or is killed, the operating system kernel automatically reaps all child processes,
    ensuring no orphaned processes remain active in memory.
  * **Isolated Crash Boundaries:** Fault tolerance is strictly isolated. If an individual agent experiences a fatal crash,
    it does not impact the main orchestrator or sibling agents. The assistant logs the failure to disk, flags
    the impacted pipeline branch, and continues running smoothly, laying the groundwork for automated agent
    hot-restarts in upcoming updates.

## Framework Stack

To maintain a clean and highly maintainable core codebase, Ovsy delegates complex lower-level operations to dedicated,
decoupled library frameworks:

| Framework   | Core Responsibility                                                                         | Technical Base           |
| :---        | :---                                                                                        | :---                     |
| **Atoman**  | Asynchronous feature management and memory-safe data ownership across threads.              | Custom Concurrency Layer |
| **AnyLM**   | Universal AI SDK abstracting cloud APIs and local inference engines into a single protocol. | Standardized SDK         |
| **Pearce**  | Ultra-lean, high-throughput TCP web server framework powering agent communication.          | Built on Axum            |
| **Cistern** | High-performance, asynchronous Retrieval-Augmented Generation (RAG) database engine.        | Built on LanceDB         |

> By keeping the core architecture slim and offloading specialized tasks to this robust underlying stack,
Ovsy offers developers and stakeholders a clean, enterprise-ready AI assistant environment optimized for speed,
predictability, and structural safety.
