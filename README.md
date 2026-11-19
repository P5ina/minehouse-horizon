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
| Minimap (official) | `mods/minimap` | nothing: client only; Horizon's own mod from horizon-rust/horizon-mods, MIT |
| Zoom (official) | `mods/zoom` | nothing: client only; Horizon's own mod from horizon-rust/horizon-mods, MIT |
| Controlling | `mods/controlling` | nothing: client only (Key Binds search) |
| MineHouse | `mods/minehouse` | MineHouse's server mods: the wardrobe (O), the anvil's real cost, the spawn zones' borders |

- `horizon.json` pins the Horizon commit (and the Nightly it came from) and horizon-mixins.
- `patches/*.patch`: our fixes to Horizon itself (`git diff` against the pinned commit), applied
  before the mods. `0001`: a sleeper keeps the position the server sends (`/lay` showed the world
  from under it).
- Horizon deletes a dated Nightly after a few days, so CI builds Horizon's other files for the
  pinned commit itself: `horizon-prepare` and the Java exporters (compiled against Minecraft
  downloaded from Mojang on the runner by `tools/fetch_minecraft.py`; only Horizon's classes leave
  it). A release carries them as `horizon-<rev12>-horizon-<platform>.zip` (`.tar.gz` on Linux),
  laid out as Horizon's own archive, and the launcher takes them from there.
- `.github/workflows/build.yml` applies the mods with horizon-mixins (`tools/stage.py`) and compiles
  `horizon-client` as the Horizon launcher's native build does. A `v*` tag publishes a release.
- Developing locally: copy the `mixins/<id>` folders into one folder and run
  `horizon-launcher native-build <folder> --trust-native-code`.

License: MIT. Plasmo Voice's client is built on [Horizon Voice Chat](https://github.com/Nirokx/horizon-voice-chat) (MIT).
Not affiliated with Mojang, Horizon, Plasmo Voice or Ping Wheel.
