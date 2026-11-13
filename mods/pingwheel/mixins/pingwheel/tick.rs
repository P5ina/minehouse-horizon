// Each client tick: register Ping Wheel's channels once a server has the
// player, send our channel (`onJoinServer`), age the pings, ping what the
// camera looks at when the key was pressed (`PingController.pollPingAction`),
// and write back changed settings.
fn pingwheel_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::pingwheel as m;
    use horizon_client::mod_api::ServerboundPlayPacket as S;
    use horizon_protocol::packets::configuration::CustomPayload;
    if !game.native_on(m::ID) {
        return;
    }
    {
        let g = &*game;
        m::with(|s| s.load(&|k| g.native_setting(m::ID, k)));
    }
    let server = game.remote.as_ref().map(|r| r.session.address.clone());
    let (Some(server), Some(_)) = (server, game.native_player()) else {
        m::with(|s| {
            s.registered = false;
            s.pings.clear();
        });
        return;
    };
    if !m::with(|s| s.registered) {
        let channel = game.native_setting(m::ID, &m::channel_key(&server)).unwrap_or_default();
        let register = S::CustomPayload(CustomPayload {
            channel: m::REGISTER.into(),
            data: m::encode_register(&[m::S2C_PING]),
        });
        if game.native_send_packet(register).is_ok() {
            let _ = game.native_send_packet(S::CustomPayload(CustomPayload {
                channel: m::C2S_CHANNEL.into(),
                data: m::encode_channel(&channel),
            }));
            m::with(|s| {
                s.registered = true;
                s.server = server.clone();
                s.channel = channel;
            });
        }
    }

    let tick = game.native_clock().map_or(0, |c| c.game_time);
    let size = game.gui_size();
    let took = m::with(|s| s.update(tick, size));
    // Which entity each entity ping follows, by UUID.
    let wanted: Vec<u128> = m::with(|s| s.pings.iter().filter_map(|p| p.ping.entity).collect());
    let mut found: Vec<(u128, u32, Option<String>)> = Vec::new();
    if !wanted.is_empty() {
        let items: Vec<(u32, String)> = game.native_client_items().into_iter().map(|i| (i.id, i.stack.item.to_string())).collect();
        let ids: Vec<u32> = game
            .native_client_entities()
            .iter()
            .map(|e| e.id)
            .chain(items.iter().map(|i| i.0))
            .collect();
        for id in ids {
            let Some(e) = game.native_entity(id) else { continue };
            if let Some(uuid) = game.native_entity_uuid(e)
                && wanted.contains(&uuid)
            {
                found.push((uuid, id, items.iter().find(|i| i.0 == id).map(|i| i.1.clone())));
            }
        }
    }
    m::with(|s| {
        for p in &mut s.pings {
            if let Some(uuid) = p.ping.entity {
                let f = found.iter().find(|f| f.0 == uuid);
                p.entity_id = f.map(|f| f.1);
                p.item = f.and_then(|f| f.2.clone());
            }
        }
    });

    if !took && m::with(|s| std::mem::take(&mut s.queued)) {
        pingwheel_ping(game, tick);
    }
    if m::with(|s| std::mem::take(&mut s.open_settings)) {
        game.native_chat("Ping Wheel settings: Options > Mods > Ping Wheel > Settings");
    }
    let store = m::with(|s| std::mem::take(&mut s.store));
    for (k, v) in store {
        game.native_set_setting(m::ID, &k, &v);
    }

    // `Raycast.traceDirectional`: the nearer of the block (through fluids while
    // crouching) and the entity (its box grown by a quarter block) along the view.
    fn pingwheel_ping(game: &mut crate::game::Game<'_>, tick: i64) {
        use crate::game::pingwheel as m;
        use horizon_client::mod_api::ServerboundPlayPacket as S;
        use horizon_protocol::packets::configuration::CustomPayload;
        let Some(cam) = game.native_camera() else { return };
        let (reach, crouching) = m::with(|s| {
            (s.config.raycast_distance.min(f64::from(s.config.ping_distance)), false)
        });
        let dir = m::view_vector(cam.yaw, cam.pitch);
        let eye = cam.position;
        let block = game.clip_ray_fluids(eye, dir, reach, crouching).map(|h| (m::distance(eye, h.point), h.point));
        let mut entity: Option<(f64, [f64; 3], u32)> = None;
        let mut boxes: Vec<(u32, [[f64; 3]; 2])> = game
            .native_client_entities()
            .into_iter()
            .filter(|e| !e.spectator)
            .map(|e| (e.id, e.bounds))
            .collect();
        boxes.extend(game.native_client_items().into_iter().map(|i| {
            let p = i.position;
            (i.id, [[p[0] - 0.125, p[1], p[2] - 0.125], [p[0] + 0.125, p[1] + 0.25, p[2] + 0.125]])
        }));
        for (id, [lo, hi]) in boxes {
            let grow = 0.25;
            let lo = [lo[0] - grow, lo[1] - grow, lo[2] - grow];
            let hi = [hi[0] + grow, hi[1] + grow, hi[2] + grow];
            if let Some(t) = m::ray_box(eye, dir, lo, hi)
                && t <= reach
                && entity.is_none_or(|e| t < e.0)
            {
                entity = Some((t, [eye[0] + dir[0] * t, eye[1] + dir[1] * t, eye[2] + dir[2] * t], id));
            }
        }
        let (pos, uuid) = match (block, entity) {
            (Some(b), Some(e)) if b.0 < e.0 => (b.1, None),
            (_, Some(e)) => (e.1, game.native_entity(e.2).and_then(|r| game.native_entity_uuid(r))),
            (Some(b), None) => (b.1, None),
            (None, None) => return,
        };
        let (channel, sequence) = m::with(|s| (s.channel.clone(), s.next_sequence(tick)));
        let ping = m::Ping {
            channel,
            pos,
            entity: uuid,
            sequence,
            dimension: m::dimension_hash(&cam.dimension),
            author: None,
        };
        let _ = game.native_send_packet(S::CustomPayload(CustomPayload {
            channel: m::C2S_PING.into(),
            data: m::encode_ping(&ping),
        }));
    }
}
