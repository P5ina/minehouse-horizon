// The schematics' missing blocks, drawn where they go (as Litematica draws
// its schematic world): nearest sections first, within a few sections of
// the camera, at most a few thousand a frame.
fn litematica_submit(game: &crate::game::Game<'_>, draw: &mut crate::game::native_hooks::NativeWorldDraw) {
    use litematica_engine as lm;
    if !game.native_on(lm::ID) {
        return;
    }
    let camera = draw.camera;
    let facts = game.res.facts();
    let world = &game.world;
    lm::with(|m| {
        if m.hidden {
            return;
        }
        let c = [(camera[0].floor() as i32) >> 4, (camera[1].floor() as i32) >> 4, (camera[2].floor() as i32) >> 4];
        let r = lm::DRAW_SECTIONS;
        let mut sections: Vec<[i32; 3]> = Vec::new();
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    sections.push([c[0] + dx, c[1] + dy, c[2] + dz]);
                }
            }
        }
        sections.sort_by_key(|s| (s[0] - c[0]).pow(2) + (s[1] - c[1]).pow(2) + (s[2] - c[2]).pow(2));
        let mut drawn = 0usize;
        for s in sections {
            for pl in m.placements.iter().filter(|p| p.shown && p.resolved) {
                let Some(list) = pl.sections.get(&s) else { continue };
                for &i in list {
                    let (pos, st) = pl.layout.blocks[i as usize];
                    if !m.layers.shows(pos[1]) || !world.chunks.contains_key(&(pos[0] >> 4, pos[2] >> 4)) {
                        continue;
                    }
                    let Some(Some(want)) = pl.ids.get(st as usize).copied() else { continue };
                    let have = world.block(pos[0], pos[1], pos[2]).0;
                    let free = facts.is_air(horizon_world::registry::BlockStateId(have))
                        || facts.states.get(have as usize).is_some_and(|f| f.replaceable);
                    if have == want || !free {
                        continue;
                    }
                    let state = &pl.layout.states[st as usize];
                    let props: Vec<(&str, &str)> = state.props.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                    draw.push();
                    if draw.translate_world([f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2])]) {
                        game.native_world_draw_block(draw, &state.name, &props);
                    }
                    draw.pop();
                    drawn += 1;
                    if drawn >= lm::DRAW_MAX {
                        return;
                    }
                }
            }
        }
    });
}
