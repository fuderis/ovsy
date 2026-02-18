# Ovsy (BETA v0.4.6): The Open-Source AI orchestrator

In a world dominated by proprietary voice assistants like Alice or Siri, imagine seizing full control. 
Enter Ovsy, an open-source AI orchestrator that's rewriting the rules for personal assistants.<br>

Developed as a lightweight core for custom AI agents, `Ovsy` handles user queries by intelligently dispatching tools 
you build yourself—turning your system into a seamless command center.<br>

Ovsy processes natural language requests and routes them to modular tools, which you develop yourself to suit your needs.<br>
But don't worry, I'm already developing a powerful `system` tool for you that will help you manage your computer, 
search the internet, and plan your schedule.<br>

> Stay tuned, it'll be ready soon; I never give up on what I start! ;)

## About `system` tool:

* **Power Management (Ready)**: Issue commands like poweroff, reboot, sleep, or lock session.
* **Music Search+Play (Ready)**: Pinpoint tracks with fuzzy, "half-word" precision (e.g., "play Disturbed"), then fire them up in your default audio player.
* **Volume Precision (Ready)**: Dial in exact levels (e.g., 75%) for audio tweaks without fumbling through menus.
* **App Launcher/Killer (Soon)**: Hunt down and launch or terminate programs by name.
* **Tasks Management (Soon)**: Set reminders/alarms, AI-powered scheduling assistance.

## Planned/Implemented:

* ✅ **LM Studio API**: Local AI models, such as `Qwen`, `Gemma`, `Llama`, and beyond..
* 🔄 **Claude API**: Anthropic's safe, powerful LLMs. Flagship `Claude 3.5 Sonnet` excels in coding, reasoning, 200K-token context.
* 🔄 **Cerebras API**: Ultra-fast LLM hosting on wafer-scale chips — 3000 tokens/sec for Llama 405B.
* 🔄 **Vosk LM**: The light-weight AI for voice recognition for implementing voice control.
* 🔄 **Whisper LM**: The medium-weight AI for voice recognition.
 
## System support:

I planned to support all systems, but I need help in the form of feedback on bugs and improvements — 
write to me if anything (contact us below).

* **Linux**: It is best supported, because I write on archlinux (it works smoothly).
* **MacOS**: It is also supported, but has not been tested yet.
* **Windows**: Partially supported, I will refine more at the end of the project development.

## Configurations:

Automatically, configs are written by paths:

* **Ovsy**: `~/.config/ovsy/settings.toml` 
* **System tool**: `~/.config/ovsy/system/config.toml`

On Windows `~/` is located in `C:\Users\UserName`.

## Installation guide (for Unix):

1. Clone the repo to a spot like /opt/ovsy:

```bash
git clone https://github.com/fuderis/ovsy.git
```

2. Build the `core` and `system` tool:

```bash
cd ovsy && cargo build --release
cd tools/system && cargo build --release
```

3. (Optional) Add a bash alias into `~/.bashrc` for one-word launches:

```bash
alias ovsy="/opt/ovsy/target/release/ovsy"
```

4. Fire up the server by command: `ovsy`. It spins the core as local server.

5. Edit the auto-generated `~/.config/ovsy/settings.toml` for ports, 
AI APIs (e.g., your LM of choice), tool paths, and more. Restart `Ovsy` server to apply.

6. Now query away: `ovsy "play music disturbed and turn off pc after 30 minutes"`. 

## Add your custom tools:

1. Just add the path to the tool dir in the `settings.toml` in the `[tools.dirs]` parameter.
2. Put the `Ovsy.toml` manifest in the root of the tool (for an example of filling, see https://github.com/fuderis/ovsy/blob/main/tools/system/Ovsy.toml)

## Ovsy isn't just software:

It's liberation for users who tired of walled gardens and the who value their anonymity.<br>

Write your own tools, drop them in, and watch the orchestrator weave them into conversational magic.
Lightning-Fast Setup on your system.

## License & Credits:

* **License**: Distributed under the [*Apache-2.0*](https://github.com/fuderis/ovsy/blob/main/LICENSE.md) license.
* **Donation**: `TON: UQB_r6IFgMYTJUKkhZNgjXcgp4aIJYwB6Gfiiukzg2lIC_Kc`
* **Contacts**:
  [*GitHub*](https://github.com/fuderis),
  [*Behance*](https://behance.net/fuderis),
  [*Telegram*](https://t.me/fuderis),
  [*Telegram Channel*](https://t.me/fuderis_club),
  [*VKontakte*](https://vk.com/fuderis).

> Thank you for your support, friends!<br>
**P.s.**: This software is actively evolving, and your suggestions and feedback are always welcome!
