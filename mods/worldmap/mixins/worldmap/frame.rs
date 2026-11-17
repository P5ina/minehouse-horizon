// Each frame: follow the world and dimension, scan the loaded chunks that are
// new or changed within a small time budget, save now and then, close the
// screen when asked, and recompose and upload the picture while it is open.
fn worldmap_frame(game: &mut crate::game::Game<'_>) {
    use crate::game::worldmap as m;
    use std::time::{Duration, Instant};
    let on = game.native_on(m::ID);
    let world = game.native_client_state().and_then(|s| s.world);
    let player = game.native_player();
    // The folder for this world and dimension.
    let dir = match (on, world, player.as_ref()) {
        (true, Some(world), Some(p)) => {
            let world = match world {
                crate::game::native_client_services::NativeWorldId::Singleplayer(s) => format!("local_{s}"),
                crate::game::native_client_services::NativeWorldId::Server(s) => format!("server_{s}"),
            };
            let dim = p.dimension.trim_start_matches("minecraft:");
            Some(game.native_data_dir(m::ID).join(m::folder_name(&world)).join(m::folder_name(dim)))
        }
        _ => None,
    };
    m::with(|s| {
        if s.store.dir() != dir.as_deref() {
            s.store.open(dir.clone());
            s.scanned.clear();
            s.south_rows.clear();
            s.view = None;
            s.shown = None;
        }
    });
    let Some(p) = player.filter(|_| dir.is_some()) else {
        if game.native_mod_screen() == Some(m::ID) {
            game.native_mod_screen_close(m::ID);
        }
        return;
    };
    // The palette, once.
    m::with(|s| {
        if s.palette.color.is_empty() {
            let facts = game.res.facts();
            for (i, f) in facts.states.iter().enumerate() {
                s.palette.color.push(f.map_color);
                let mut b = match f.fluid {
                    horizon_client::blocks::Fluid::None => 0,
                    horizon_client::blocks::Fluid::Water => 1,
                    horizon_client::blocks::Fluid::Lava => 2,
                };
                if facts.is_air(horizon_world::registry::BlockStateId(i as u32)) {
                    b |= m::AIR;
                }
                if f.sturdy_full >> 1 & 1 != 0 {
                    b |= m::STURDY_TOP;
                }
                s.palette.flags.push(b);
            }
        }
    });
    // Scan: nearest chunks first, new ones and changed ones, within 2 ms.
    let started = Instant::now();
    let budget = Duration::from_millis(2);
    let (pcx, pcz) = ((p.x.floor() as i32) >> 4, (p.z.floor() as i32) >> 4);
    let radius = game.settings.render_distance.clamp(2, 32) + 1;
    let head = (p.y + 1.62).floor() as i32;
    let layer = if game.world.shape.has_ceiling { m::Layer::Below(head) } else { m::Layer::Surface };
    let revision = game.world.revision;
    let air = game.world.air.0;
    m::with(|s| {
        s.scanned.retain(|k, _| game.world.chunks.contains_key(k));
        'rings: for ring in 0..=radius {
            for (dx, dz) in m::ring_offsets(ring) {
                let key = (pcx + dx, pcz + dz);
                let Some(chunk) = game.world.chunks.get(&key) else {
                    continue;
                };
                let stale = match s.scanned.get(&key) {
                    None => true,
                    Some(&(rev, at)) => rev != revision && at.elapsed() >= m::RESCAN,
                };
                if !stale {
                    continue;
                }
                let bottom = chunk.min_y();
                let top = (0..chunk.sections.len())
                    .rev()
                    .find(|&i| !chunk.sections[i].is_uniform(air))
                    .map_or(bottom, |i| bottom + i as i32 * 16 + 15);
                let north = s.south_rows.get(&(key.0, key.1 - 1)).copied();
                let (cols, south) = m::scan_chunk(
                    |x, y, z| chunk.get(x, y, z),
                    &s.palette,
                    top,
                    bottom,
                    layer,
                    north,
                    key,
                );
                s.store.put_chunk(key.0, key.1, &cols);
                s.south_rows.insert(key, south);
                s.scanned.insert(key, (revision, Instant::now()));
                if started.elapsed() >= budget {
                    break 'rings;
                }
            }
        }
        if s.south_rows.len() > 8192 {
            s.south_rows.retain(|k, _| (k.0 - pcx).abs() <= 64 && (k.1 - pcz).abs() <= 64);
        }
        if s.saved.is_none_or(|t| t.elapsed() >= m::SAVE_EVERY) {
            s.store.save();
            s.store.trim(p.x as i32, p.z as i32);
            s.saved = Some(Instant::now());
        }
    });
    // The screen.
    if m::with(|s| std::mem::take(&mut s.close)) {
        game.native_mod_screen_close(m::ID);
    }
    if game.native_mod_screen() != Some(m::ID) {
        // Drawn again on the next open: the slot may have been rebuilt.
        m::with(|s| s.shown = None);
        return;
    }
    let Some(size) = game.native_client_state().map(|s| s.gui_size) else {
        return;
    };
    let image = m::with(|s| {
        if s.view.is_none() || std::mem::take(&mut s.recenter) {
            let zoom = s.view.map_or(m::DEFAULT_ZOOM, |v| v.zoom);
            s.view = Some(m::View { x: p.x, z: p.z, zoom });
        }
        let view = s.view?;
        if !s.store.changed && s.shown == Some((view, size)) {
            return None;
        }
        s.store.changed = false;
        s.shown = Some((view, size));
        let (w, h, _) = m::image_size(size);
        Some((w, h, m::compose(&mut s.store, view, size, &horizon_client::blocks::MAP_COLORS)))
    });
    let Some((w, h, rgba)) = image else {
        return;
    };
    let Some(rect) = game.gui.atlas.sprites.get(horizon_client::hud::MOD_IMAGE_SPRITE).copied() else {
        return;
    };
    let (aw, ah) = (game.gui.atlas.width as f32, game.gui.atlas.height as f32);
    game.map_patches.push(horizon_client::font::Patch {
        x: (rect.u0 * aw).round() as u32,
        y: (rect.v0 * ah).round() as u32 + m::IMAGE_Y,
        width: w,
        height: h,
        rgba,
    });
}
