# MineHouse for Horizon

A native [Horizon](https://github.com/horizon-rust/horizon) mod: the parts of the MineHouse client mod
(`minehouse-client.jar`, `menu-mod/` in minehouse-landing) that a Horizon player needs on MineHouse.

- **Anvil:** the real enchantment cost from the server (`minehouse:anvil_cost`), with no "Too Expensive!";
  red when you don't have the levels.
- **Spawn zones:** the zone borders the server sends (`minehouse:zone_shapes`): walls, top and bottom lines,
  a line every 10 blocks, corner posts and the zone's name; the admin's draft with numbered corners.

Both channels are registered (`minecraft:register`) once the server has the player, which is how the
server mods learn the client has them (and stop the action-bar cost and the particles).

Not ported (yet): the main menu and pack updater (the launcher does that), the wardrobe screen, the Rift
mode's client and the show editor.

License: MIT.
