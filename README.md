# Ovsy — Ultra-Fast AI Assistant Kernel

![Cover](/assets/cover.png)

**The high-density, low-latency multi-agent engine engineered for social networks, enterprise chat platforms, and private local AI assistants.**
> Engineered to power millions of chat messages without burning your budget or server RAM.<br>

Built in asynchronous Rust, Ovsy discards bulky network abstractions (MCP, JSON-RPC over TCP/HTTP) in favor of sub-millisecond local IPC.
It is purpose-built to process thousands of concurrent chat queries instantly, drastically cut token expenditure, and run seamlessly on minimal hardware.

> **⚠️ EXPERIMENTAL:** Ovsy is undergoing rapid architectural evolution. Interfaces and IPC contracts may break between commits.
Source audit is recommended before production deployment.

## Key Features

* **Sub-Millisecond Response Latency:** Powered by pure Unix Domain Sockets (`AF_UNIX`).
  Eliminates loopback network stacks to stream responses to chat clients with zero delay.

* **Extreme Token & Cost Savings:** Subagents receive isolated, surgical subtask context instead of bloated histories.
  Combined with an embedded JS engine (`Boa`) for local math/parsing, Ovsy slashes your LLM API bills.

* **Zero Attention Decay (No Hallucinations):** Strict context isolation ensures subagents never get lost in massive chat logs,
  keeping automated bot responses precise, reliable, and deterministic.

* **0 MB Idle Footprint:** Thanks to a 2-phase CLI lifecycle (`metadata` ➔ `serve`), background agents consume **zero RAM when idle**.
  Spin up thousands of subagents without frying server resources.

* **Self-Healing Stream Engine:** Auto-recovers from stream breaks, LLM output errors, or malformed JSON arguments in real time,
  guaranteeing uninterrupted chat streams for users.

* **Enterprise-Grade Isolation & Safety:** Native OS-level process management (`PR_SET_PDEATHSIG`, RAII drops) guarantees zero zombie processes
  and isolated memory boundaries for custom subagent workflows.

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

## Requirements

* **Supported OS:** Unix-like operating systems  (`Linux`, `macOS`, `BSD`).
* **Rust toolchain:** `cargo` and `rustc` (2024 edition or newer)
* **JSON processor:** `jq` (required by `build.sh`)

1. Installing Rust

```bash
# 1. Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install and set nightly toolchain
rustup toolchain install nightly
rustup default nightly
```

2. Installing JQ

* **Arch Linux:** `sudo pacman -S jq`
* **Ubuntu / Debian:** `sudo apt install -y jq`
* **macOS:** `brew install jq`

## Installation

1. Clone the repository and run the build script:
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
