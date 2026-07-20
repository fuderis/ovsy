# Ovsy — Ultra-Fast AI Kernel (Experimental)

![Cover](/assets/cover.png)

Ovsy is a low-level, high-performance assistant kernel for multi-agent AI systems, engineered on top of the asynchronous Rust runtime.
The project completely discards universal industry abstraction standards (such as the Model Context Protocol — MCP)
and traditional network overhead (JSON-RPC over TCP/HTTP) in favor of sub-millisecond inter-process communication (IPC).<br>

> **WARNING:** This project is currently undergoing intensive refactoring, deep testing, and experimental development.
Kernel interfaces and IPC protocol specifications are subject to breaking changes.
It is not recommended for production environments without prior source code auditing.

## Orchestration Architecture

![Scheme](/assets/scheme.png)

The kernel coordinates user queries through a multi-stage orchestration engine governed by a central execution loop (handle_query).

> `User Query` ➔ `Orchestrator Evaluation` ➔ `Concurrent Task Spawning` ➔ `Self-Healing Loop / Resolution`

## Key Architectural Advantages

1. Two-Phase Agent CLI Lifecycle (Zero-Idle)

To eliminate initialization lags, the lazy /init endpoint has been completely deprecated. Inter-agent coordination has been refactored into a deterministic command-line interface (CLI) powered by clap:

  * **metadata Phase:** The kernel invokes the agent binary once. The agent instantly dumps its configuration,
  system prompt, and skill tree into stdout (JSON format) and exits immediately. In idle mode, memory consumption is exactly 0 MB RAM.

  * **serve Phase:** Upon task graph activation, the kernel executes the agent as a long-running IPC daemon,
  initializing a local socket in memory to begin processing data streams immediately.

2. Concurrent Engine and Self-Healing Loop

  * **Parallel Queues:** The Orchestration Engine translates user prompts into a directed dependency tree.
  Tasks without upstream locks are parallelized instantly across isolated Tokio worker threads.

  * **Context Isolation:** To prevent model attention decay, background agents receive only the localized description
  of their subtask and the precise payload results from parent nodes in the dependency graph.

  * **Self-Healing Loop:** The kernel runtime intercepts empty LLM responses, hallucinations, or malformed JSON arguments
  from downstream tools in real time. The error trace is wrapped back into the system context,
  triggering an immediate retry pass without bringing down the pipeline.

3. Unix-Native IPC and Process Resilience

The network loopback stack is fully bypassed to minimize system call overhead.

  * **Socket Security:** Communication is routed exclusively through local Unix Domain Sockets (AF_UNIX)
    located at /tmp/ovsy/uds/*.sock, with access control managed via POSIX file permissions.

  * **Anti-Zombie Guarantees:** Agent processes utilize RAII concepts and are bound to the kernel with kill_on_drop(true).

  * **Low-Level OS Hooks:** On Linux, the kernel applies PR_SET_PDEATHSIG combined with SIGKILL via a libc::prctl abstraction
    before execution. On macOS, a dedicated stdin monitor thread intercepts EOF events if the orchestrator terminates unexpectedly,
    ensuring clean socket removal from the filesystem.

4. Inline Expression Evaluation (Boa Engine)

To offload minor calculation passes from the LLM, a native Rust JavaScript interpreter `Boa Engine` is embedded directly
into the task loop. Deterministic algorithms (arithmetic, timestamps, string transformations) are evaluated
instantly in-memory and routed directly to dependent nodes by their task_id,
significantly reducing unnecessary LLM generation cycles.

## Ecosystem Components

The core kernel externalizes infrastructural operations into specialized, zero-overhead crates:

  * **Atoman:** Thread-safe asynchronous state and core kernel configuration management.
  * **AnyLM:** A unified SDK abstraction layer routing inference to cloud APIs or local execution nodes (such as LM Studio).
  * **Pearce:** An ultra-lean asynchronous HTTP/IPC routing framework built on top of Axum.
  * **Cistern:** High-performance vector retrieval (LanceDB) paired with a transactional Key-Value engine (Sled).

> By keeping the core architecture slim and offloading specialized tasks to this robust underlying stack,
Ovsy offers developers and stakeholders a clean, enterprise-ready AI assistant environment optimized for speed,
predictability, and structural safety.

## Quick Test Drive

Compilation and deployment utilize standard Cargo toolchains. Windows environments are not supported.
Bash

1. Clone and automatically build the release using the script
```bash
git clone https://github.com/fuderis/ovsy.git && cd ovsy
bash build.sh
```

2. View all available CLI commands
```bash
ovsy --help
```

## License & Feedback

> This software is distributed under the [GPL 3.0](https://github.com/fuderis/ovsy/blob/main/LICENSE.md) license.

You can contact me via [GitHub](https://github.com/fuderis) or send a message to my [E-Mail](mailto:synapdrake@ya.ru).
Contributions, bug reports, feature requests, and feedback are always welcome.<br>

*We invite you to participate in testing, finding edge cases, and optimizing Unix IPC pipelines.<br>
Pull Requests and Issues are greatly appreciated!*
