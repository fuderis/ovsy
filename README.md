# Ovsy — The Open‑Source AI Orchestrator

> Version: 0.6.1 BETA-TESTING

![Header](/header.png)

In a world dominated by proprietary voice assistants like Alice and Siri, imagine taking back full control.
Meet Ovsy — an open‑source AI orchestrator that redefines what a personal assistant can be.
Fully anonymous, fully yours — you decide how to use this power.<br>

Designed as a lightweight core for custom AI agents, Ovsy intelligently routes your natural‑language commands
to modular tools you build yourself, turning your system into a seamless command center.<br>

![Demo](/demo.mp4)

## Features:

* **AI Agent Task Delegation**: Distributes tasks to custom AI agents that you create yourself.
* **Automatic Agent Restart**: Automatically restarts AI agents upon rebuild or update without downtime.
* **Async Server Mode**: Runs in fully asynchronous mode and can be used as a production server.
* **Demo AI Agents Included**: Comes with ready-to-use demonstration agents for testing — Music Agent and System Power Management.

## Planned/Implemented:

### Basic tools:

* ✅ **Power Management (100%)**: Issue commands like poweroff, reboot, sleep, or lock session.
* ✅ **Music Search+Play (100%)**: Pinpoint tracks with fuzzy, "half-word" precision (e.g., "play Disturbed"), then fire them up in your default audio player.
* ⚙️ **Volume Precision (50%)**: Dial in exact levels (e.g., 75%) for audio tweaks without fumbling through menus.
* ⏳ **App Launcher/Killer (0%)**: Hunt down and launch or terminate programs by name.
* ⏳ **Tasks Management (0%)**: Set reminders/alarms, AI-powered scheduling assistance.
* ⏳ **Web-Search (0%)**: A search assistant for news, weather, currency, stock information, developer tools, and etc.

### AI APIs:

* ✅ **LM Studio (100%)**: Local AI models, such as `Qwen`, `Gemma`, `Llama`, and beyond..
* ✅ **Anthropic (100%)**: Anthropic's safe, powerful LLMs. Flagship `Claude`, `Sonnet` and `Haiku` excels in coding, reasoning, 200K-token context.
* ✅ **Cerebras (100%)**: Ultra-fast LLM hosting on wafer-scale chips — 3000 tokens/sec for `Llama`, `GPT OSS`, `Qwen3`, `Zai GLM`.
 
## System support:

* **Linux**: It's full supported.
* **MacOS**: It's also supported, but maybe more tests are needed..
* **Windows**: Partially supported.

## Configurations:

Automatically, configs are written by paths:

* **Ovsy**: `~/.config/ovsy/settings.toml` 
* **Agents**: `~/.config/ovsy/agents/[AGENT_NAME]/config.toml`

On Windows `~/` is located in `C:\Users\UserName`.

## Installation guide (for Unix):

1. Clone the repo and build project:

```bash
git clone https://github.com/fuderis/ovsy.git && cd ovsy
bash install.sh
```
By default, it will be installed in the `/opt/ovsy` directory (you can change it in the `INSTALL_DIR` constant in the `install.sh` file)

2. (Optional) Add a bash alias into `~/.bashrc` for one-word launches:

```bash
alias ovsy="/opt/ovsy/ovsy"
```

3. Fire up the server by command: `ovsy`. It spins the core as local server.

4. Edit the auto-generated `~/.config/ovsy/settings.toml` for ports, 
AI APIs (e.g., your LM of choice), tool paths, and more. Restart `Ovsy` server to apply.

5. Now query away: `ovsy "play music Disturbed and turnoff system after 30 minutes"`. 

## Add your custom agents:

1. Write your agent in `/opt/ovsy/agents` dir or add the custom path to the agents parent dir in the `settings.toml` in the `[agents.scan_dirs]` parameter.
2. Put the `Ovsy.toml` manifest in the root of the agent and configure it (see examples in system agents on `.../ovsy/agents/[AGENT_NAME]/Ovsy.toml`).

## Ovsy isn't just software:

It's liberation for users who tired of walled gardens and the who value their anonymity.<br>

Write your own tools, drop them in, and watch the orchestrator weave them into conversational magic.
Lightning-Fast Setup on your system.

## License & Credits:

* **License**: Distributed under the [*Apache-2.0*](https://github.com/fuderis/ovsy/blob/main/LICENSE.md) license.
* **Contacts**:
  [*GitHub*](https://github.com/fuderis),
  [*Behance*](https://behance.net/fuderis),
  [*Telegram*](https://t.me/fuderis),
  [*Telegram Channel*](https://t.me/fuderis_club),
  [*VKontakte*](https://vk.com/fuderis).
* **For a cup of coffee**: `[TON] UQBq2GVLt_nu6bF8ku0RneWDr_B0AdrBMXVPcRrNmTU6mz65`
> Thank you for your support! =)<br>

**P.s.**: This software is actively evolving, and your suggestions and feedback are always welcome!
