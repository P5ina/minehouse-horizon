# MineHouse Horizon

[Horizon](https://github.com/horizon-rust/horizon) (Minecraft: Java Edition 26.3 in Rust, MIT) with the
client mods MineHouse players need, built once in CI for macOS, Windows and Linux. Players get it from
the MineHouse launcher and never compile anything.

| Mod | Folder | Talks to |
|---|---|---|
| Ping Wheel | `mods/pingwheel` | the server's Ping Wheel 1.12 (Fabric) |
| Plasmo Voice | `mods/plasmovoice` | the server's Plasmo Voice 2.1.x |
| AppleSkin | `mods/appleskin` | the server's AppleSkin 3.0 (saturation, exhaustion) |
| Chat Heads | `mods/chatheads` | nothing: client only |
| Fast Trading | `mods/fasttrading` | nothing: client only (the trading window) |
| Simple AutoClicker | `mods/autoclicker` | nothing: client only |
| World Map | `mods/worldmap` | nothing: client only |
| Litematica + Printer | `mods/litematica` | nothing; Syncmatica's shared schematics when the server has it |
| Controlling | `mods/controlling` | nothing: client only (Key Binds search) |
| MineHouse | `mods/minehouse` | MineHouse's server mods: the anvil's real cost, the spawn zones' borders |

- `horizon.json` pins the Horizon commit (and the Nightly whose files the build reuses) and horizon-mixins.
- `.github/workflows/build.yml` applies the mods with horizon-mixins (`tools/stage.py`) and compiles
  `horizon-client` as the Horizon launcher's native build does. A `v*` tag publishes a release.
- Developing locally: copy the `mixins/<id>` folders into one folder and run
  `horizon-launcher native-build <folder> --trust-native-code`.

License: MIT. Plasmo Voice's client is built on [Horizon Voice Chat](https://github.com/Nirokx/horizon-voice-chat) (MIT).
Not affiliated with Mojang, Horizon, Plasmo Voice or Ping Wheel.
