# Horizon Voice Chat

A native Horizon mod (Minecraft 26.3, Horizon Nightly 2026-11-08, `fdc87ae`) that is a client for
the [Simple Voice Chat](https://github.com/henkelmax/simple-voice-chat) protocol. It uses the same
protocol as the Java mod, so on any server with the voice chat mod or plugin you can talk with players
who play on the regular Java launcher with Simple Voice Chat.

- **Menu (V)**: everything without chat commands. The left side has the connection's status, buttons
  for the microphone, voice chat on/off, push to talk or voice activation and the group names, `-`/`+`
  for voice volume, microphone gain and the activation level (with a live microphone meter), and
  **Hotkeys**: every key of the mod as it is bound now. The right side lists the server's groups
  with their members: **Join** (a password group asks for its password), **Leave** for yours, and
  **New group** (name, optional password, type, Create). Tab moves between fields, Enter submits,
  Escape or V closes, the wheel scrolls the groups.
- **Talking**: hold **Caps Lock** (push to talk), or switch to voice activation. Voices come from where
  the players stand: they get quieter with distance and pan left or right, as the Java mod's do.
- **Keys** (Options > Mods > Horizon Voice Chat): the menu (V), push to talk (Caps Lock), whisper (not
  bound), mute the microphone (J), voice chat off/on (K). The menu shows whatever you bind them to.
- **Groups**: each player's group shows above their head (`[Builders]`; yours in green, others in
  yellow). Your group and its members are on screen, and `/vc groups` lists every group with its members.
  Join, leave and create them in the menu, or with `/vc join <group> [password]`, leave with `/vc leave`, make one with
  `/vc create <name> [password] [normal|open|isolated]`. The server's own `/voicechat` command works too.
- **On screen** (top left): connecting, talking or whispering, muted, voice chat off, your group, and
  who is talking.
- **`/vc`**: `status`, `mute`/`unmute`, `off`/`on`, `mode ptt|voice`, `volume <0-200>`,
  `volume <player> <0-400>`, `gain <10-500>`, `threshold <dB>`, and the group commands above.
- **Settings page**: activation, microphone, voice chat, voice volume, microphone gain, voice activation
  level, protocol, status on screen, group names above players.

Both Simple Voice Chat protocols are supported: **2.6.x** (compatibility version 20, the current
releases) and **2.7** (21, the pre-release). Automatic asks with 2.6.x only, because a server can kick a
client that asks with a version it doesn't speak. For a 2.7 server, set Settings > Protocol to 2.7.

It needs **libopus** (the `opus` package; already installed on most Linux desktops) and uses the
default microphone and speakers.

## Not affiliated

This is an unofficial project. It is not made, endorsed or supported by Max Henkel or the Simple
Voice Chat project; please don't report its problems to them. "Simple Voice Chat" is named to say
which mod and plugin it works with.

## Where the protocol code comes from

This is **not** a clean-room implementation. The mod was written with an AI coding assistant
(Claude Code), which read [Simple Voice Chat's source](https://github.com/henkelmax/simple-voice-chat)
on GitHub (its packet classes, `AudioChannel`, `MicThread` and some config defaults) to implement the
protocol. No code is copied verbatim (this is Rust, Simple Voice Chat is Java), but the protocol and
audio code follow Simple Voice Chat's implementation, and comments name its classes. Simple Voice Chat
is "Copyright (c) Max Henkel - All Rights Reserved". If you are its author and want this changed or
taken down, open an issue and it will be.

## Build and install

Uses the Horizon launcher's own Rust and Horizon source (nothing else to install):

```sh
L=~/.local/share/horizon-launcher
export PATH=$L/rust/1.99.1-x86_64-unknown-linux-gnu/bin:$PATH CARGO_HOME=$L/cargo
(cd mixins/voicechat/engine && cargo test --offline && rm -f Cargo.lock)   # the engine's tests
python3 tools/package.py --install        # target/voicechat-1.0.1.hnmod, copied into the launcher's native mods
```

Then build the game with its mods in the launcher. The mod is pinned to Horizon `fdc87ae` because it
reads the server's custom payloads and the other players' positions inside the game's multiplayer code
(`game::remote`), which the stable hook points don't offer. A newer Nightly may need the pin in
`compatibility.json` and `mixins/voicechat/horizon-mixin.toml` moved.

## Layout

- `mixins/voicechat/engine/`: the mod's own crate (built once and cached), with its unit tests
  - `proto.rs`: Simple Voice Chat's packets, both versions; `buf.rs`: `FriendlyByteBuf`
  - `crypt.rs`: AES-128-GCM, as `Secret` encrypts; `opus.rs`: libopus, loaded at run time
  - `net.rs`: the UDP connection (authenticate, check, keep alive) and the mic thread
  - `mix.rs`: jitter buffers, distance and panning, resampling; `audio.rs`: microphone and speakers (cpal)
  - `ui.rs`: the menu; `labels.rs`: group names above players; `lib.rs`: the session, keys, HUD, `/vc` and settings
- `mixins/voicechat/*.rs`: one small hook adapter per hook point
- `mixins/voicechat/horizon-mixin.toml`: where each hook goes
- `tools/package.py`: makes the `.hnmod`

## Tested

- Engine unit tests: the packet formats of both versions, the AES-GCM test vector, Opus through the
  system libopus, the mixer, distance and panning, the session, the group display, and the menu
  (hotkeys and groups shown, joining a password group, typing a new group).
- The bundle applies to Horizon `fdc87ae` with horizon-mixins 0.6.0, and the modded client
  type-checks (`cargo check -p horizon-client`).
- Live, against Paper 26.3 with the Simple Voice Chat 2.6.27 plugin: two clients driven by this engine
  authenticated and connected to voice chat; one heard the other's voice (a test tone through
  Opus and the server); creating and joining groups, the member lists and the labels above players all
  came from the server.
- Not tested yet: the menu on screen in the real game, 2.7 servers live (no 2.7 plugin build is published), a real microphone and speakers
  in the game, and talking with the Java mod itself.

## License

MIT, see [LICENSE](LICENSE), for the parts that are this project's own work. Simple Voice Chat itself is
not covered by this license and remains its author's, including whatever of its design the protocol
code follows (see above).
