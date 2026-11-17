# Litematica for Horizon

A native [Horizon](https://github.com/horizon-rust/horizon) mod (Minecraft 26.3) for building from Litematica
schematics, written for Horizon (no code of Litematica's or Printer's): it reads `.litematic` files from the game
folder's `schematics/`, places them in the world, draws the blocks still missing where they go, marks wrong
blocks, lists the materials still needed, and its printer places the missing blocks within reach from the
hotbar.

- `/lm list`, `/lm load <name>`: place a schematic at your feet. `/lm here`, `/lm move <x> <y> <z>`,
  `/lm nudge <up|down|north|...|forward|left> [n]`, `/lm rotate [ccw]`, `/lm mirror <none|x|z>` move and turn it.
- `/lm layer <all|y|here|below y|above y>`: which layers show and print.
- `/lm materials`: what is still needed against what you carry.
- `/lm printer` or **Caps Lock**: the printer. It places one block every two ticks within 4.5 blocks, looking
  the way the block needs (stairs, furnaces, pistons, logs, slabs, trapdoors, wall torches, buttons...).
- `/lm show`, `/lm hide`, `/lm info`, `/lm select <n>`, `/lm unload [all]`.

Light blue boxes are missing blocks, red ones wrong blocks, orange ones the right block in the wrong state.
Placements are kept per server and world.

**Syncmatica.** On a server with Syncmatica (0.3), the schematics players shared there are listed by
`/lm shared`; `/lm shared <n>` downloads one into `schematics/syncmatica/` (checked against its hash) and places
it where and how it stands on the server, in its own dimension. Moves on the server follow; a placement removed
there stays here as your own. Sharing and moving shared placements from Horizon are not offered. The protocol
is Syncmatica's (CC0), written anew here.

License: MIT.
