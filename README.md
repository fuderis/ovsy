<p align="center">
  <img src="https://raw.githubusercontent.com/fuderis/ovsy/main/assets/logo.png" alt="Ovsy" width="80" />
</p>

<h2 align="center">Ovsy Kernel</h2>
<p align="center">
  <strong>A Unix-native orchestration kernel for lightweight AI assistants.</strong><br>
  Built in Rust with a focus on predictable execution, low latency, and efficient LLM orchestration.
</p>

---

<img src="https://raw.githubusercontent.com/fuderis/ovsy/main/assets/cover.png" alt="Cover" width="100%" />

Ovsy is an orchestration kernel for local and server-side AI assistants.<br>

Instead of building on top of HTTP, JSON-RPC, or heavyweight orchestration frameworks, Ovsy uses native Unix IPC,
centralized task scheduling, and isolated worker processes to execute AI workflows with minimal overhead.<br>

The result is a system optimized for predictable execution, low latency, and efficient LLM usage instead of unrestricted orchestration flexibility.<br>

> **⚠️ EXPERIMENTAL:** Ovsy is undergoing rapid architectural evolution. Interfaces and IPC contracts may break between commits.
Source audit is recommended before production deployment.


## Design Principles

### 1. Kernel-owned orchestration

**The kernel is the only component responsible for planning and scheduling work.**<br>


Agents never communicate directly with each other. They only receive their own task description and the outputs of dependent tasks,
making execution deterministic and easier to reason about.

### 2. Task-scoped context

**Each agent receives only the context required for its task instead of the complete conversation history.**<br>

This reduces prompt size, keeps responsibilities isolated, and avoids unnecessary context propagation between independent tasks.

### 3. Persistent IPC workers

**Agents run as long-lived Unix IPC services.**<br>

Once activated, workers remain alive as IPC services instead of being spawned for every request.
This removes repeated process startup overhead and keeps request latency consistent under sustained load.

### 4. Minimal LLM orchestration

**Ovsy intentionally minimizes orchestration calls.**<br>

A typical request requires only:

  1. planning the task graph;
  2. executing the graph;
  3. aggregating results and deciding whether another iteration is necessary.

Additional model invocations occur only during bounded self-healing retries.

### 5. Native process lifecycle

**The kernel owns the lifecycle of every worker process.**<br>

Child workers are attached to the kernel process. Unexpected worker failures can be recovered independently,
while kernel termination automatically cleans up all child processes, preventing orphaned services and zombie processes.<br>

* If a worker exits unexpectedly, it can be restarted independently.
* If the kernel terminates, all child processes terminate with it, preventing orphaned background services.

### 6. Unix-native IPC

**Communication happens through Unix Domain Sockets instead of network protocols whenever all components run on the same machine.**<br>

This removes unnecessary networking layers and avoids unnecessary networking and serialization overhead.


## Architecture

<img src="https://raw.githubusercontent.com/fuderis/ovsy/main/assets/scheme.png" alt="Scheme" width="100%" />

Ovsy follows a centralized orchestration model.

> `User Query` ➔ `Orchestrator Evaluation` ➔ `Concurrent Task Spawning` ➔ `Self-Healing Loop / Resolution`

The kernel evaluates the user request, builds a dependency graph, schedules independent tasks concurrently, aggregates their outputs,
and decides whether another execution cycle is required.<br>

Agents are treated as isolated workers rather than autonomous decision-makers.


### Architectural Decisions

#### 1. Two-phase lifecycle

**Problem:** Keeping every agent running wastes memory.<br>
**Solution:** The metadata phase extracts static information once. The serve phase starts only when the kernel activates the worker.

#### 2. Centralized orchestration

**Problem:** Recursive agent communication quickly becomes expensive and difficult to control.<br>
**Solution:** The kernel exclusively owns orchestration. Workers never coordinate directly.

#### 3. Self-healing execution

**Problem:** LLMs occasionally produce malformed tool arguments or incomplete responses.<br>
**Solution:** The runtime retries failed execution with structured error feedback up to a configurable retry limit instead of aborting the pipeline.

#### 4. Native IPC

**Problem:** Loopback networking introduces unnecessary serialization and system-call overhead for local assistants.<br>
**Solution:** Workers communicate through Unix Domain Sockets.

#### 5. Embedded expression engine

**Problem:** Simple deterministic work should not require an LLM.<br>
**Solution:** JavaScript runtime evaluates expressions locally.

#### 6. Process lifecycle

**Problem:** Worker processes must not outlive the orchestrator.<br>
**Solution:** Workers are tied to the kernel lifecycle through native operating system primitives, ensuring deterministic cleanup and recovery.

#### 7. Long-lived workers

**Problem:** Launching a process for every request introduces unnecessary latency.<br>
**Solution:** Workers become persistent IPC services after activation and immediately accept new requests without repeated initialization.


## Scope

### Designed for

Ovsy is built for lightweight AI assistants and automation workflows:

* Local desktop assistants
* Chat platforms and bots
* Slack integrations
* API automation
* Computer control
* Server-side assistant orchestration

### Design Model

Ovsy uses centralized orchestration.<br>

Agents are isolated task executors managed by the kernel. They do not communicate directly with each other or create new agents.
All planning, scheduling, and context distribution are controlled by the kernel.

### Not a Generic Agent Runtime

Ovsy is not designed to run existing autonomous agent frameworks or MCP agents without adaptation.<br>

It intentionally favors predictable execution, low latency, reduced context size, and lower operational overhead over unrestricted agent autonomy.


## Roadmap

The following features are planned for the next development cycle (approximately the next 2–3 months).

### Long-term Memory (RAG)

Persistent user memory with dynamic fact storage, editing, and retrieval.
Integrated directly into the orchestrator rather than implemented as a standalone agent.

### Personal Context

Per-user persistent configuration managed by the orchestrator.<br>

**Planned capabilities include:**

  * custom system prompts
  * optional user profile information (e.g. location, timezone, preferences)
  * dynamic long-term memory (RAG)
  * personal task management (TODOs and reminders)

All data will be isolated per user and injected into execution only when relevant.

### Web Search

Native web search powered by the lightweight **Obscure** browser.
Search will be available as a built-in orchestrator capability instead of an external agent.

### CLI Improvements

A complete rewrite of the interactive CLI chat experience to improve usability and reliability.


## Ecosystem Components

The core kernel externalizes infrastructural operations into specialized crates:

  * **Atoman:** Thread-safe asynchronous state and core kernel configuration management.
  * **AnyLM:** A unified SDK abstraction layer routing inference to cloud APIs or local execution nodes (such as LM Studio).
  * **Pearce:** An ultra-lean asynchronous HTTP/IPC routing framework built on top of Axum.
  * **Cistern:** High-performance vector retrieval (LanceDB) paired with a transactional Key-Value engine (Sled).

> Ovsy keeps the orchestration kernel intentionally small. Infrastructure concerns such as inference, storage, routing,
and state management are implemented as independent crates that can evolve without increasing kernel complexity.


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
