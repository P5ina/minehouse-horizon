# Zoom (Horizon native mod)

Horizon's official Zoom: hold a key (Z by default) to zoom in, and turn the mouse wheel while you hold it to zoom
further in or out. Its settings page (Options > Mods > Zoom > Settings) has the key and three options: the
starting zoom (2x to 10x), smooth zoom (eased like the spyglass) and slower turning while zoomed.

It is a [horizon-mixins](https://github.com/horizon-rust/horizon-mixins) bundle (`mixins/zoom/`) that the Horizon
launcher builds into the game. It keeps the settings of earlier versions: the same files
(`horizon-mods/.horizon-mod-data/zoom/level`, `smooth`, `sensitivity`) and the same key line
(`key.zoom.zoom:…` in `horizon-mods.txt`).

## Install

In the Horizon launcher: Mods → Zoom → Install, then **Build** on the same page (once built, switch it
on or off in the game under Options → Mods). The
launcher downloads its own Rust and builds the game with your native mods; see Horizon's
[native mods](https://github.com/horizon-rust/horizon/blob/main/docs/native-mods.md).

## Made on the hook points

The bundle hooks only Horizon's native hook points (`crates/horizon-client/src/game/native_hooks.rs`), which keep
their names and signatures across releases. `compatibility.json` names the version it needs (`"hooks": 1`), so the
launcher builds Zoom on any Horizon with that version or a later one, the player's own game by default, together with any other
mod. `horizon_revision` is only the commit it was last tested on.

| File | What it is |
|---|---|
| `mixins/zoom/horizon-mixin.toml` | The bundle: one module and the HEAD hooks |
| `mixins/zoom/runtime.rs` | The mod's logic and state, added to the game as `game::official_zoom`; also this crate's library, for its tests |
| `mixins/zoom/*.rs` | One small helper per hook |
| `../tools/native.py` | Shared by the native mods: apply on Horizon (from GitHub, by commit), check, build, package the `.hnmod`, run the proof |
| `proof.py` (with `../tools/game.py`) | The proof: the built game driven with `horizon-ctl --offscreen` on fresh, temporary data |

## Build and prove

```sh
(cd native/zoom && cargo test)                    # the logic
python3 native/tools/native.py check zoom          # apply on the tested Horizon and cargo check the game
python3 native/tools/native.py check zoom --horizon latest  # ... on the newest Nightly
python3 native/tools/native.py check all           # with the other native mods, as a player's set
python3 native/tools/native.py package zoom        # native/target/zoom-<version>.hnmod for the registry
python3 native/tools/native.py proof zoom --resources /path/to/.local/horizon-client/26.3
```

The proof needs resources prepared from your own Minecraft 26.3 (`horizon-prepare`); none are in this repository.
It seeds an earlier version's settings (8x, no smoothing, key C) in a temporary game folder, then checks that
Options > Mods shows them, that Z does nothing and C zooms, that the wheel zooms instead of moving the hotbar
while C is held, and that the page's buttons write the same files. CI runs `cargo test`, packages the `.hnmod`
and checks the game on the tested commit, and every Monday on the newest Nightly.

License: MIT.

Vendored from horizon-rust/horizon-mods@87cbc72 (native/zoom), MIT; pinned to MineHouse Horizon's revision.
