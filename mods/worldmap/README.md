# World Map for Horizon

A native [Horizon](https://github.com/horizon-rust/horizon) mod (Minecraft 26.3): a full-screen map of every
chunk the player has seen, in the spirit of Xaero's World Map but written from scratch (no code or art of
Xaero's). Chunks are scanned as map items see them, with their colours and height shading, while you play,
and kept on disk per server (or single-player world) and dimension in
`horizon-mods/.horizon-mod-data/worldmap/`.

- **M** opens and closes the map (Options > Mods to rebind).
- Drag with the left button to move, the wheel zooms about the pointer, Space puts you back in the middle.
- Other players the client can see show as green dots with their names; the coordinates under the pointer
  are in the bottom left.

In the Nether the map shows the floor below your head, as the minimap does.

License: MIT.
