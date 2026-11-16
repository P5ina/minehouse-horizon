# AppleSkin for Horizon

A native [Horizon](https://github.com/horizon-rust/horizon) mod (Minecraft 26.3): a port of
[AppleSkin](https://github.com/squeek502/AppleSkin) 3.0 for Fabric (public domain).

- The saturation outline over the food bar and the exhaustion bar under it (from the server's AppleSkin:
  `appleskin:saturation`, `appleskin:exhaustion`, `appleskin:natural_regeneration`).
- Holding food: the hunger, saturation and hearts it would restore, flashing.
- Food tooltips: the hunger and saturation a food gives.

`mixins/appleskin/runtime.rs` is generated: edit `tools/runtime_head.rs`, then run
`tools/gen_icons.py <AppleSkin>/textures/icons.png`, which appends AppleSkin's icons as pixel runs.

License: Unlicense, as AppleSkin.
