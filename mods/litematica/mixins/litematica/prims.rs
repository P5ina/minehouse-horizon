// Litematica's overlay over the schematics: a light blue box on a missing
// block, red on a wrong block, orange on the right block in the wrong state,
// and each placement's outline (the selected one yellow).
fn litematica_prims(game: &crate::game::Game<'_>, camera: [f64; 3], primitives: &mut horizon_client::world_primitives::WorldPrimitives) {
    use litematica_engine as lm;
    if !game.native_on(lm::ID) {
        return;
    }
    let facts = game.res.facts();
    let world = &game.world;
    lm::with(|m| {
        if m.hidden {
            return;
        }
        for (n, pl) in m.placements.iter().enumerate().filter(|(_, p)| p.shown) {
            let (lo, hi) = (pl.layout.min, pl.layout.max);
            let colour = if n == m.selected { 0xFFFFE040 } else { 0xA0FFFFFF };
            primitives.box_outline(
                [f64::from(lo[0]), f64::from(lo[1]), f64::from(lo[2])],
                [f64::from(hi[0]) + 1.0, f64::from(hi[1]) + 1.0, f64::from(hi[2]) + 1.0],
                colour,
                2.0,
            );
        }
        let c = [(camera[0].floor() as i32) >> 4, (camera[1].floor() as i32) >> 4, (camera[2].floor() as i32) >> 4];
        let r = lm::DRAW_SECTIONS;
        let mut boxes = 0usize;
        for pl in m.placements.iter().filter(|p| p.shown && p.resolved) {
            for dx in -r..=r {
                for dy in -r..=r {
                    for dz in -r..=r {
                        let Some(list) = pl.sections.get(&[c[0] + dx, c[1] + dy, c[2] + dz]) else { continue };
                        for &i in list {
                            let (pos, st) = pl.layout.blocks[i as usize];
                            if !m.layers.shows(pos[1]) || !world.chunks.contains_key(&(pos[0] >> 4, pos[2] >> 4)) {
                                continue;
                            }
                            let Some(Some(want)) = pl.ids.get(st as usize).copied() else { continue };
                            let have = world.block(pos[0], pos[1], pos[2]).0;
                            if have == want {
                                continue;
                            }
                            let free = facts.is_air(horizon_world::registry::BlockStateId(have))
                                || facts.states.get(have as usize).is_some_and(|f| f.replaceable);
                            let colour = if free {
                                0x3040C0FF
                            } else if game.native_state_block(horizon_world::registry::BlockStateId(have))
                                == game.native_state_block(horizon_world::registry::BlockStateId(want))
                            {
                                0x60FF9000
                            } else {
                                0x60FF2020
                            };
                            let e = 0.002;
                            let lo = [f64::from(pos[0]) - e, f64::from(pos[1]) - e, f64::from(pos[2]) - e];
                            let hi = [f64::from(pos[0]) + 1.0 + e, f64::from(pos[1]) + 1.0 + e, f64::from(pos[2]) + 1.0 + e];
                            primitives.filled_box(lo, hi, colour);
                            if !free {
                                primitives.box_outline(lo, hi, colour | 0xFF000000, 2.0);
                            }
                            boxes += 1;
                            if boxes >= lm::DRAW_MAX {
                                return;
                            }
                        }
                    }
                }
            }
        }
    });
}
