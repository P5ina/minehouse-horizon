# MineHouse for Horizon

A native [Horizon](https://github.com/horizon-rust/horizon) mod: the parts of the MineHouse client mod
(`minehouse-client.jar`, `menu-mod/` in minehouse-landing) that a Horizon player needs on MineHouse.

- **Anvil:** the real enchantment cost from the server (`minehouse:anvil_cost`), with no "Too Expensive!";
  red when you don't have the levels.
- **Spawn zones:** the zone borders the server sends (`minehouse:zone_shapes`): walls, top and bottom lines,
  a line every 10 blocks, corner posts and the zone's name; the admin's draft with numbered corners.

- **Wardrobe (O):** the shop of hats and backpacks (`minehouse-cosmetics:state` / `:action`): slot tabs,
  tiles with the server pack's models, wear / take off / buy, the 16 dyes, the balance and "Пополнить".
  The Fabric screen's 3D look at yourself is not ported. Your own hat and backpack come from the server the
  vanilla way (helmet slot, passenger): `minehouse-cosmetics:no_self` asks for that (Cosmetics 0.6.1+).

- **End of Chapter 1:** when the server sends `minehouse:chapter_end` (Events' `swallow`, right before it
  kicks everyone and stops), the title turns black with the dot in place of the logo, no splash and no
  Realms; Singleplayer and Multiplayer stay. The mark is `horizon-mods/.horizon-mod-data/minehouse/chapter-end.txt`
  and goes once MineHouse lets the player in again, at least 5 minutes after the end (as the Fabric mod's
  `ChapterEnd`). The pull into the hole itself is the server's vanilla post effects, which Horizon applies.

The channels are registered (`minecraft:register`) once the server has the player, which is how the
server mods learn the client has them (and stop the action-bar cost and the particles).

Not ported (yet): the rest of the main menu and the pack updater (the launcher does that), the Rift mode's client and the
show editor.

License: MIT.
