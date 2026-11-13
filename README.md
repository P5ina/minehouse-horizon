# MineHouse Horizon

[Horizon](https://github.com/horizon-rust/horizon) (Minecraft: Java Edition 26.3 in Rust, MIT) with the
client mods MineHouse players need, built once in CI for macOS, Windows and Linux. Players get it from
the MineHouse launcher and never compile anything.

| Mod | Folder | Talks to |
|---|---|---|
| Ping Wheel | `mods/pingwheel` | the server's Ping Wheel 1.12 (Fabric) |
| Plasmo Voice | `mods/plasmovoice` | the server's Plasmo Voice 2.1.x |

- `horizon.json` pins the Horizon commit (and the Nightly whose files the build reuses) and horizon-mixins.
- `.github/workflows/build.yml` applies the mods with horizon-mixins (`tools/stage.py`) and compiles
  `horizon-client` as the Horizon launcher's native build does. A `v*` tag publishes a release.
- Developing locally: copy the `mixins/<id>` folders into one folder and run
  `horizon-launcher native-build <folder> --trust-native-code`.

License: MIT. Plasmo Voice's client is built on [Horizon Voice Chat](https://github.com/Nirokx/horizon-voice-chat) (MIT).
Not affiliated with Mojang, Horizon, Plasmo Voice or Ping Wheel.
