# Ovsy — The Open‑Source AI Orchestrator

> **Version:** 0.6.5 BETA-testing<br>

![Header](/header.png)

In a world dominated by proprietary, data-harvesting voice assistants, imagine taking back full control. 
Meet **Ovsy** — an open‑source AI orchestrator that redefines what a personal assistant can be. Fully anonymous, fully private, and completely yours.
You decide how to harness this power.

Designed as a lightweight core for custom AI agents, Ovsy intelligently routes your natural‑language commands to modular tools you build yourself,
turning your system into a seamless, conversational command center.

![Demo](/demo.gif)

---

## ✨ Key Features

* 🤖 **AI Agent Task Delegation**: Distributes complex tasks to custom-tailored AI agents that you create.
* 🔄 **Hot-Reload & Auto-Restart**: Automatically restarts AI agents upon rebuild or update without taking the core server down.
* ⚡ **High-Performance Async Core**: Runs in a fully asynchronous environment, capable of handling production-grade loads.
* 📦 **Out-of-the-Box Demos**: Comes with ready-to-use demonstration agents including a System Music Agent and Power Management.

## 💻 OS Support Matrix

Ovsy is built for cross-platform freedom. Version 0.6.5 brings complete, unified support across all major operating systems.

| Feature / Module      | Progress | 🐧 Linux | 🍎 macOS | 🪟 Windows | Description                                               |
| :---                  | :---:    | :---:    | :---:    | :---:      | :---                                                      |
| **Core Orchestrator** | `100%`   | ✅       | ✅       | ✅         | High-performance async routing engine.                    |
| **Agent Hot-Reload**  | `100%`   | ✅       | ✅       | ✅         | Zero-downtime agent restarts on update.                   |
| **Custom Agent API**  | `100%`   | ✅       | ✅       | ✅         | Standardized manifest for modular tools.                  |
| **Power Management**  | `100%`   | ✅       | ✅       | ✅         | Poweroff, reboot, sleep, and session locking.             |
| **Music Control**     | `100%`   | ✅       | ✅       | ✅         | Fuzzy search and playback in default players.             |
| **Volume Precision**  | `100%`   | ✅       | ✅       | ✅         | Exact percentage-based audio adjustments.                 |
| **App Launcher**      | `0%`     | ⏳       | ⏳       | ⏳         | Native program execution and termination.                 |
| **Task Management**   | `0%`     | ⏳       | ⏳       | ⏳         | AI scheduling, reminders, and alarms.                     |
| **Web Search**        | `0%`     | ⏳       | ⏳       | ⏳         | Live retrieval for news, weather, and tools.              |

### 🧠 Supported AI Backends

* 🏠 **LM Studio** (`100%` ✅): Run local models entirely offline (e.g., `Qwen`, `Gemma`, `Llama`).
* 🦅 **Anthropic** (`100%` ✅): Industry-leading reasoning and coding via `Claude`, `Sonnet`, and `Haiku`.
* 🚀 **Cerebras** (`100%` ✅): Ultra-fast inference on wafer-scale chips (up to 3000 tokens/sec).

---

## ⚙️ Configuration

Ovsy values transparency. All configurations are stored in plain text and are easily accessible:

* **Ovsy Core:** `~/.config/ovsy/settings.toml`
* **Agents:** `~/.config/ovsy/agents/[AGENT_NAME]/config.toml`

*(On Windows, the user directory `~/` corresponds to `C:\Users\UserName`)*

---

## 🚀 Installation Guide (Unix)

### 1. Clone and Build
Clone the repository and run the automated build script. By default, it installs to `/opt/ovsy` (you can modify the `INSTALL_DIR` constant inside `install.sh`).

```bash
git clone https://github.com/fuderis/ovsy.git && cd ovsy
bash build.sh
```

### 2. Add an Alias (Optional)

For seamless, one-word executions from any terminal directory, add an alias to your `~/.bashrc` or `~/.zshrc`:
```bash
alias ovsy="/opt/ovsy/ovsy"
```

### 3. Spin up the Core

Fire up the server to spin the core as a local background daemon:
```bash
ovsy
```

### 4. Configure

Edit the auto-generated `~/.config/ovsy/settings.toml` to link your preferred LLM APIs, adjust server ports, and toggle features. 
Restart the Ovsy server to apply the changes.

### 5. Start Querying
```bash
ovsy "play music Disturbed and turn off the system after 30 minutes"
```

## 🔌 Create Your Own Agents

Building for Ovsy is designed to be painless:
 * Create a directory for your agent in `/opt/ovsy/agents` (or add a custom path under `[agents.scan_dirs]` in your `settings.toml`).
 * Drop an `Ovsy.toml` manifest file into the root of your agent's folder.
 * Use the pre-existing system agents at `.../ovsy/agents/(AGENT_NAME)/Ovsy.toml` as blueprints to get started.

---

## 🕊️ Ovsy is more than software

It is digital liberation for those tired of walled gardens. Write your own tools, drop them in, and watch the orchestrator weave them into conversational magic.
Fast, private, and endlessly expandable.

## 📜 License & Credits:

* **License**: Distributed under the [*Apache-2.0*](https://github.com/fuderis/ovsy/blob/main/LICENSE.md) license.
* **Contacts**:
  [*GitHub*](https://github.com/fuderis),
  [*Behance*](https://behance.net/fuderis),
  [*Telegram*](https://t.me/fuderis),
  [*Telegram Channel*](https://t.me/fuderis_club),
  [*VKontakte*](https://vk.com/fuderis).
* **For a cup of coffee**: 
> Thank you for your support! =)<br>

**P.s.**: This software is actively evolving, and your suggestions and feedback are always welcome!

## 📜 License & Credits

Distributed under the **Apache-2.0** License. See [LICENSE](https://github.com/fuderis/ovsy/blob/main/LICENSE.md) for more information.

### 📩 Project Feedback

* **Code & Projects:** [GitHub](https://github.com/fuderis)
* **Design Portfolio:** [Behance](https://behance.net/fuderis)
* **Personal Contact:** [Telegram](https://t.me/fuderis)
* **Community & Updates:** [Telegram Channel](https://t.me/fuderis_club) | [VKontakte Group](https://vk.com/fuderis_club)

### ☕ Support the Project

If Ovsy helps you reclaim your digital autonomy and you want to support its active evolution, consider buying me a cup of coffee:

> 💎 **TON Wallet:** `UQBq2GVLt_nu6bF8ku0RneWDr_B0AdrBMXVPcRrNmTU6mz65`<br>

<br>
> P.S.: This software is actively evolving. Your suggestions, bug reports, and feedback are always welcome!
