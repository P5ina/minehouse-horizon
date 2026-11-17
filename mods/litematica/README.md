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
Placements are kept per server and world. Syncmatica's shared placements are not supported yet.

License: MIT.
