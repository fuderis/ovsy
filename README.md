# Ovsy — The Open‑Source AI Orchestrator

> Version: 0.5.3 BETA

![Logo](logo.png)

In a world dominated by proprietary voice assistants like Alice and Siri, imagine taking back full control.
Meet Ovsy — an open‑source AI orchestrator that redefines what a personal assistant can be.
Fully anonymous, fully yours — you decide how to use this power.<br>

Designed as a lightweight core for custom AI agents, Ovsy intelligently routes your natural‑language commands
to modular tools you build yourself, turning your system into a seamless command center.<br>


## Planned/Implemented:

### Basic tools:

* ✅ **Power Management (100%)**: Issue commands like poweroff, reboot, sleep, or lock session.
* ✅ **Music Search+Play (100%)**: Pinpoint tracks with fuzzy, "half-word" precision (e.g., "play Disturbed"), then fire them up in your default audio player.
* ⚙️ **Volume Precision (50%)**: Dial in exact levels (e.g., 75%) for audio tweaks without fumbling through menus.
* ⏳ **App Launcher/Killer (0%)**: Hunt down and launch or terminate programs by name.
* ⏳ **Tasks Management (0%)**: Set reminders/alarms, AI-powered scheduling assistance.
* ⏳ **Web-Search (0%)**: A search assistant for news, weather, currency, stock information, developer tools, and etc.

### LM's API:

* ✅ **LM Studio (100%)**: Local AI models, such as `Qwen`, `Gemma`, `Llama`, and beyond..
* ✅ **Anthropic (100%)**: Anthropic's safe, powerful LLMs. Flagship `Claude`, `Sonnet` and `Haiku` excels in coding, reasoning, 200K-token context.
* ✅ **Cerebras (100%)**: Ultra-fast LLM hosting on wafer-scale chips — 3000 tokens/sec for `Llama`, `GPT OSS`, `Qwen3`, `Zai GLM`.
* ⏳ **Vosk (0%)**: The light-weight AI for voice recognition for implementing voice control.
* ⏳ **Whisper (0%)**: The medium-weight AI for voice recognition.
 
## System support:

I planned to support all systems, but I need help in the form of feedback on bugs and improvements — 
write to me if anything (contact us below).

* **Linux**: It is best supported, because I write on archlinux (it works smoothly).
* **MacOS**: It is also supported, but has not been tested yet.
* **Windows**: Partially supported, I will refine more at the end of the project development.

## Configurations:

Automatically, configs are written by paths:

* **Ovsy**: `~/.config/ovsy/settings.toml` 
* **Agents**: `~/.config/ovsy/agents/[AGENT_NAME]/config.toml`

On Windows `~/` is located in `C:\Users\UserName`.

## Installation guide (for Unix):

1. Clone the repo and build project:

```bash
git clone https://github.com/fuderis/ovsy.git
cd ovsy && bash build.sh
```

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
