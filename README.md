# Ovsy (BETA v0.3.0): The Open-Source AI orchestrator

In a world dominated by proprietary voice assistants like Alice or Siri, imagine seizing full control. 
Enter Ovsy, an open-source AI orchestrator that's rewriting the rules for personal assistants.<br><br>

Developed as a lightweight core for custom AI agents, Ovsy handles user queries by intelligently dispatching tools 
you build yourself—turning your system into a seamless command center.<br>
Ovsy processes natural language requests and routes them to modular "tools."

## Out of the box, it ships with `pc-control`, a powerhouse for desktop domination:
  - **Power Management**: Issue commands like poweroff, sleep, lock, or cancel poweroff, complete with customizable timeouts. Perfect for "shut down in 30 minutes" workflows.
  - **Smart Music Search**: Pinpoint tracks with fuzzy, "half-word" precision (e.g., "play Disturbed"), then fire them up in your default audio player. Includes a clean "stop music" command.
  - **Volume Precision**: Dial in exact levels (e.g., 75%) for audio tweaks without fumbling through menus.
  - **App Launcher/Killer**: Hunt down and launch—or terminate—programs by name.<br><br>

## Ovsy shines for tinkerers:

Write your own Rust-based tools, drop them in, and watch the orchestrator weave them into conversational magic.
Lightning-Fast Setup on system.<br><br>

## Installation guide (for Linux):

1. Clone the repo to a spot like /opt/ovsy:

```text
git clone https://github.com/fuderis/ovsy.git
```

2. Build the core and pc-control tool (root rights may be required):

```text
cd ovsy && cargo build --release
cd tools/pc-control && cargo build --release
```

3. Add a bash alias to ~/.bashrc for one-word launches:

```bash
    alias ovsy="/opt/ovsy/target/release/ovsy"
```

4. Fire up the server by command: `ovsy`. It spins the core as local server.

5. Edit the auto-generated `~/.local/share/ovsy/config/settings.toml` for ports, 
AI APIs (e.g., your LLM of choice), tool paths, and more. Restart to apply.

6. Now query away: `ovsy "play music disturbed and turn off pc after 30 minutes"`, 
followed by `ovsy "cancel power off"`. 

## Ovsy isn't just software:

It's liberation for Linux users tired of walled gardens. Fork it, extend it, own it.

## License & Credits:

- **License**: Distributed under the [*Apache-2.0*](https://github.com/fuderis/ovsy/blob/main/LICENSE.md) license.
- **Contacts**:
  [GitHub](https://github.com/fuderis),
  [Behance](https://behance.net/fuderis),
  [Telegram](https://t.me/fuderis)
  [TG Channel](https://t.me/fuderis_club),
  [VKontakte](https://vk.com/fuderis)

**P.s.**: This software is actively evolving, and your suggestions and feedback are always welcome!
