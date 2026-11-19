// Each client tick: the placements of the world joined (loaded and saved
// under its name), each placement's block states as the game's, and the
// printer: one missing block within reach a few ticks, from the hotbar.
fn litematica_tick(game: &mut crate::game::Game<'_>) {
    use litematica_engine as lm;
    if !game.native_on(lm::ID) {
        return;
    }
    // The world: its placements come back when it is joined again.
    let world = game.native_client_state().and_then(|s| s.world).map(|w| match w {
        crate::game::native_hooks::NativeWorldId::Singleplayer(s) => format!("local_{s}"),
        crate::game::native_hooks::NativeWorldId::Server(s) => format!("server_{s}"),
    });
    let dir = game.native_instance_dir().join("schematics");
    let changed = lm::with(|m| m.world != world);
    if changed {
        let saved = world.as_ref().and_then(|w| game.native_setting(lm::ID, &lm::world_key(w)));
        lm::with(|m| {
            m.placements = saved.map(|t| lm::load_text(&t.replace("\\n", "\n"), &dir)).unwrap_or_default();
            m.selected = m.placements.len().saturating_sub(1);
            m.world = world.clone();
            m.printer = false;
            // Syncmatica starts again with the next server's handshake.
            m.sync = lm::sync::Sync::default();
            m.wanted = None;
        });
    }
    // Syncmatica's requests (`/lm shared`).
    for data in lm::with(|m| std::mem::take(&mut m.sync.out)) {
        game.send(horizon_protocol::packets::play::Serverbound::CustomPayload(
            horizon_protocol::packets::configuration::CustomPayload { channel: lm::sync::CHANNEL.into(), data },
        ));
    }
    // Placements made in another dimension show only there.
    if let Some(dim) = game.native_player().map(|p| p.dimension) {
        lm::with(|m| {
            for p in &mut m.placements {
                p.shown = p.dimension.as_deref().is_none_or(|d| d == dim);
            }
        });
    }
    if let Some(w) = world.as_ref() {
        if let Some(text) = lm::with(|m| std::mem::take(&mut m.save).then(|| lm::save_text(m))) {
            game.native_set_setting(lm::ID, &lm::world_key(w), &text.replace('\n', "\\n"));
        }
    }
    // The states, as the game knows them (older files upgraded first).
    lm::with(|m| {
        for p in m.placements.iter_mut().filter(|p| !p.resolved) {
            let version = p.schematic.data_version;
            p.ids = p
                .layout
                .states
                .iter()
                .map(|s| {
                    let (name, props) = game.native_upgrade_block_state(&s.name, &s.props, version);
                    let registry = &game.pack.assets.registry;
                    let mut state = registry.default_state(&name).and_then(|id| registry.state(id)).cloned()?;
                    for (k, v) in &props {
                        if let Some(slot) = state.properties.get_mut(k.as_str()) {
                            *slot = v.clone();
                        }
                    }
                    registry.id(&state).or_else(|| registry.default_state(&name)).map(|id| id.0)
                })
                .collect();
            p.resolved = true;
        }
    });
    // The printer.
    let ready = lm::with(|m| {
        if !m.printer || m.hidden {
            return false;
        }
        if m.cooldown > 0 {
            m.cooldown -= 1;
            return false;
        }
        true
    });
    if !ready || !game.native_playing() || game.remote.is_none() {
        return;
    }
    if matches!(
        game.mode,
        horizon_client::command::args::GameMode::Spectator | horizon_client::command::args::GameMode::Adventure
    ) {
        return;
    }
    let Some(p) = game.native_player() else { return };
    let on_ground = game.player.as_ref().is_some_and(|pl| pl.ground.on_ground);
    let eye = [p.x, p.y + 1.62, p.z];
    let facts = game.res.facts();
    let world = &game.world;
    let loaded = |q: [i32; 3]| world.chunks.contains_key(&(q[0] >> 4, q[2] >> 4));
    let free = |id: u32| {
        let f = facts.states.get(id as usize);
        facts.is_air(horizon_world::registry::BlockStateId(id)) || f.is_some_and(|f| f.replaceable)
    };
    let solid = |q: [i32; 3]| {
        if !loaded(q) {
            return false;
        }
        let id = world.block(q[0], q[1], q[2]).0;
        if free(id) {
            return false;
        }
        let Some(f) = facts.states.get(id as usize) else { return false };
        if f.block_entity {
            return false;
        }
        let name = game.native_state_block(horizon_world::registry::BlockStateId(id)).unwrap_or("");
        !(name.ends_with("_door")
            || name.ends_with("_trapdoor")
            || name.ends_with("_fence_gate")
            || name.ends_with("_button")
            || name.ends_with("_bed")
            || name.ends_with("anvil")
            || matches!(
                name,
                "minecraft:crafting_table"
                    | "minecraft:lever"
                    | "minecraft:repeater"
                    | "minecraft:comparator"
                    | "minecraft:note_block"
                    | "minecraft:cartography_table"
                    | "minecraft:smithing_table"
                    | "minecraft:grindstone"
                    | "minecraft:loom"
                    | "minecraft:stonecutter"
                    | "minecraft:daylight_detector"
                    | "minecraft:redstone_wire"
            ))
    };
    // The hotbar slot holding an item.
    let hotbar = |item: &str| (0..9).find(|&i| game.inventory.get(i).and_then(|s| s.as_ref()).is_some_and(|s| &*s.item == item));
    let choice = lm::with(|m| {
        let layers = m.layers;
        let reach = lm::REACH + 1.0;
        let mut best: Option<(f64, [i32; 3], lm::schematic::State, u32, usize)> = None;
        let lo = [((eye[0] - reach).floor() as i32) >> 4, ((eye[1] - reach).floor() as i32) >> 4, ((eye[2] - reach).floor() as i32) >> 4];
        let hi = [((eye[0] + reach).floor() as i32) >> 4, ((eye[1] + reach).floor() as i32) >> 4, ((eye[2] + reach).floor() as i32) >> 4];
        for pl in m.placements.iter().filter(|pl| pl.shown && pl.resolved) {
            for sx in lo[0]..=hi[0] {
                for sy in lo[1]..=hi[1] {
                    for sz in lo[2]..=hi[2] {
                        let Some(list) = pl.sections.get(&[sx, sy, sz]) else { continue };
                        for &i in list {
                            let (pos, s) = pl.layout.blocks[i as usize];
                            if !layers.shows(pos[1]) || !loaded(pos) {
                                continue;
                            }
                            let d = (0..3).map(|a| (f64::from(pos[a]) + 0.5 - eye[a]).powi(2)).sum::<f64>().sqrt();
                            if d > lm::REACH || best.as_ref().is_some_and(|b| b.0 <= d) {
                                continue;
                            }
                            let Some(Some(want)) = pl.ids.get(s as usize).copied() else { continue };
                            let have = world.block(pos[0], pos[1], pos[2]).0;
                            if have == want || !free(have) {
                                continue;
                            }
                            // Not where the player stands.
                            let inside = (f64::from(pos[0]) < p.x + 0.3 && f64::from(pos[0]) + 1.0 > p.x - 0.3)
                                && (f64::from(pos[2]) < p.z + 0.3 && f64::from(pos[2]) + 1.0 > p.z - 0.3)
                                && (f64::from(pos[1]) < p.y + 1.8 && f64::from(pos[1]) + 1.0 > p.y);
                            if inside {
                                continue;
                            }
                            let state = &pl.layout.states[s as usize];
                            let item = facts
                                .states
                                .get(want as usize)
                                .map(|f| f.item.to_string())
                                .filter(|i| !i.is_empty() && i != "minecraft:air")
                                .unwrap_or_else(|| lm::item_of(&state.name).to_owned());
                            let Some(slot) = hotbar(&item).or_else(|| hotbar(lm::item_of(&state.name))) else { continue };
                            best = Some((d, pos, state.clone(), want, slot));
                        }
                    }
                }
            }
        }
        best
    });
    let Some((_, pos, state, _, slot)) = choice else { return };
    let Some(click) = lm::plan(&state, pos, p.yaw, &solid) else { return };
    // Look, choose the slot, click.
    use horizon_protocol::packets::play;
    if let Some((yaw, pitch)) = click.look {
        game.send(play::Serverbound::MovePlayerRot(play::MovePlayerRot { yaw, pitch, flags: u8::from(on_ground) }));
    }
    if game.selected != slot {
        game.selected = slot;
        game.send(play::Serverbound::SetCarriedItem(play::SetCarriedItem { slot: slot as i16 }));
        if let Some(r) = game.remote.as_ref() {
            r.carried.set(Some(slot));
        }
    }
    let face = match click.face {
        [0, -1, 0] => 0,
        [0, 1, 0] => 1,
        [0, 0, -1] => 2,
        [0, 0, 1] => 3,
        [-1, 0, 0] => 4,
        _ => 5,
    };
    let sequence = match game.remote.as_mut() {
        Some(r) => {
            r.sequence += 1;
            r.sequence
        }
        None => return,
    };
    game.send(play::Serverbound::UseItemOn(play::UseItemOn {
        hand: 0,
        pos: click.pos,
        face,
        cursor: click.cursor,
        inside: false,
        world_border: false,
        sequence,
    }));
    lm::with(|m| m.cooldown = lm::PRINT_EVERY);
}
