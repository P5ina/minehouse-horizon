// Plasmo Voice's HUD, as its renderers draw it: the activation icon above the
// hotbar (`HudIconRenderer`), the overlay of who is talking in the top left
// (`OverlayRenderer`), and each player's icon above their head
// (`EntityIconRenderer`), seen from this frame's camera.
fn voicechat_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    use horizon_client::extras::Corner;
    use plasmovoice_engine as vc;
    if !game.native_on(vc::ID) {
        return;
    }
    let (w, h) = (batch.width, batch.height);
    let sprite = |game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, icon: &str, dst: [f32; 4]| {
        if let Some(src) = vc::icons::cell(icon) {
            game.native_sprite(batch, "horizon/modimage", src, dst);
        }
    };
    // The voice distance (`DistanceVisualizeRenderer`): a ring on the
    // ground at that radius, for two seconds after it changed.
    if let (Some((radius, argb, at)), Some(cam), Some(me)) = (vc::visualize(), game.native_camera(), game.native_player()) {
        let centre = at.unwrap_or([me.x, me.y + 0.05, me.z]);
        let n = 96;
        let mut prev: Option<[f32; 2]> = None;
        for i in 0..=n {
            let a = i as f64 / n as f64 * std::f64::consts::TAU;
            let p = [centre[0] + a.cos() * f64::from(radius), centre[1], centre[2] + a.sin() * f64::from(radius)];
            let here = vc::labels::project_matrix(p, cam.position, &cam.view, &cam.projection, (w, h));
            if let (Some(a), Some(b)) = (prev, here) {
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let len = (dx * dx + dy * dy).sqrt();
                if len < w {
                    let steps = (len / 1.5).ceil().max(1.0) as usize;
                    for k in 0..steps {
                        let t = k as f32 / steps as f32;
                        game.native_fill(batch, [a[0] + dx * t - 0.75, a[1] + dy * t - 0.75, 1.5, 1.5], argb);
                    }
                }
            }
            prev = here;
        }
    }
    // Icons above heads, up to 64 blocks away.
    if let Some(cam) = game.native_camera() {
        let offset = vc::icon_offset();
        let tan = (cam.fov_degrees.to_radians() * 0.5).tan().max(1e-3);
        for e in game.native_client_entities() {
            let Some(uuid) = e.player_uuid else { continue };
            if e.invisible {
                continue;
            }
            let Some(icon) = vc::player_icon(uuid) else { continue };
            // The name tag's attachment, the tag itself (0.3), then half a block.
            let at = [e.position[0] + offset[0], e.bounds[1][1] + 0.3 + 0.5 + offset[1], e.position[2] + offset[2]];
            let d = [at[0] - cam.position[0], at[1] - cam.position[1], at[2] - cam.position[2]];
            let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if dist * dist > 4096.0 {
                continue;
            }
            let Some([x, y]) = vc::labels::project_matrix(at, cam.position, &cam.view, &cam.projection, (w, h)) else { continue };
            // 10 units at 0.025 blocks each: a quarter block wide.
            let size = (0.25 / (dist as f32 * tan * 2.0) * h).clamp(4.0, 64.0);
            sprite(game, batch, icon, [x - size / 2.0, y - size, size, size]);
        }
    }
    // The activation icon: 16 by 16, centred, 32 pixels above the bottom.
    let (icon_at, overlay_at) = vc::positions();
    if let Some(icon) = vc::hud_icon() {
        let [x, y] = icon_at.place([w, h], 16.0, 16.0);
        sprite(game, batch, icon, [x, y, 16.0, 16.0]);
    }
    // The overlay: who talks now, each with their line's icon.
    let corner = match overlay_at {
        vc::Position::TopRight => Corner::TopRight,
        vc::Position::BottomLeft => Corner::BottomLeft,
        vc::Position::BottomRight => Corner::BottomRight,
        _ => Corner::TopLeft,
    };
    if game.native_corner_shown(corner) {
        for (name, icon) in vc::overlay() {
            let tw = game.native_text_width(&name);
            let (x, y) = corners.place(corner, tw + 16.0, 12.0);
            game.native_fill(batch, [x, y, tw + 16.0, 12.0], 0x4000_0000);
            sprite(game, batch, icon, [x + 1.0, y + 1.0, 10.0, 10.0]);
            game.native_text(batch, &name, [x + 13.0, y + 2.0], 0xFF_FFFF, true);
        }
    }
    // A connection problem, as Plasmo tells it in chat, stays visible here.
    for line in vc::hud(&game.native_mod_key(vc::ID, vc::KEY_PTT, vc::DEFAULT_PTT)).into_iter().filter(|l| l.rgb == 0xFF5555) {
        let tw = game.native_text_width(&line.text) + 4.0;
        let (x, y) = corners.place(Corner::TopLeft, tw, 12.0);
        game.native_fill(batch, [x, y, tw, 12.0], 0x9000_0000);
        game.native_text(batch, &line.text, [x + 2.0, y + 2.0], line.rgb, true);
    }
}
