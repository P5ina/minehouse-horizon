// The map screen: the picture over the whole screen, the other players as
// dots with their names, the player as an arrow, the title, the coordinates
// under the pointer and how to move about.
fn worldmap_render(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
) {
    use crate::game::worldmap as m;
    if game.native_mod_screen() != Some(m::ID) {
        return;
    }
    let size = [batch.width, batch.height];
    let Some((view, shown)) = m::with(|s| s.shown) else {
        game.native_fill(batch, [0.0, 0.0, size[0], size[1]], 0xFF0E0E12);
        return;
    };
    let (w, h, per) = m::image_size(shown);
    game.native_fill(batch, [0.0, 0.0, size[0], size[1]], 0xFF0E0E12);
    game.native_sprite(
        batch,
        horizon_client::hud::MOD_IMAGE_SPRITE,
        [0.0, m::IMAGE_Y as f32, w as f32, h as f32],
        [0.0, 0.0, w as f32 * per, h as f32 * per],
    );
    // Other players the client knows of.
    let own = game.native_player();
    for e in game.native_client_entities() {
        let Some(name) = e.player_name.as_deref() else { continue };
        if e.invisible || e.spectator {
            continue;
        }
        if let Some(p) = own.as_ref() {
            if (e.position[0] - p.x).abs() < 1e-3 && (e.position[2] - p.z).abs() < 1e-3 {
                continue;
            }
        }
        let [x, y] = view.point_of(e.position[0], e.position[2], size);
        if x < -4.0 || y < -4.0 || x > size[0] + 4.0 || y > size[1] + 4.0 {
            continue;
        }
        game.native_fill(batch, [x - 2.5, y - 2.5, 5.0, 5.0], 0xFF000000);
        game.native_fill(batch, [x - 1.5, y - 1.5, 3.0, 3.0], 0xFF55FF55);
        let tw = game.native_text_width(name);
        game.native_fill(batch, [x - tw / 2.0 - 1.0, y - 13.0, tw + 2.0, 10.0], 0x80000000);
        game.native_text(batch, name, [x - tw / 2.0, y - 12.0], 0xFFFFFF, false);
    }
    // The player: an arrow pointing where they look.
    if let Some(p) = own {
        let [cx, cy] = view.point_of(p.x, p.z, size);
        let yaw = f64::from(p.yaw).to_radians();
        // Yaw 0 faces south (+z, down on the map), 90 west (-x).
        let (dx, dy) = (-yaw.sin() as f32, yaw.cos() as f32);
        let (nx, ny) = (-dy, dx);
        for (outline, argb) in [(1.0f32, 0xFF000000u32), (0.0, 0xFFFFFFFF)] {
            for i in 0..=14 {
                let t = i as f32 / 14.0;
                // From the tip back to the base: wider towards the base.
                let along = 5.0 - t * 9.0;
                let half = t * 3.5;
                let mut k = -half;
                while k <= half {
                    let x = cx + dx * along + nx * k;
                    let y = cy + dy * along + ny * k;
                    game.native_fill(batch, [x - 0.75 - outline, y - 0.75 - outline, 1.5 + outline * 2.0, 1.5 + outline * 2.0], argb);
                    k += 1.0;
                }
            }
        }
        game.native_fill(batch, [cx - 1.0, cy - 1.0, 2.0, 2.0], 0xFFFF3030);
    }
    // Texts.
    let russian = horizon_client::lang::code().starts_with("ru");
    let title = m::tr(russian, "title");
    let tw = game.native_text_width(title);
    game.native_fill(batch, [size[0] / 2.0 - tw / 2.0 - 4.0, 4.0, tw + 8.0, 13.0], 0x90000000);
    game.native_text(batch, title, [size[0] / 2.0 - tw / 2.0, 7.0], 0xFFFFFF, true);
    let (bx, bz) = view.block_at(screen.mouse, size);
    let coords = format!("X: {}  Z: {}", bx.floor() as i64, bz.floor() as i64);
    let hint = m::tr(russian, "hint");
    let y = size[1] - 14.0;
    let cw = game.native_text_width(&coords);
    game.native_fill(batch, [2.0, y - 3.0, cw + 8.0, 13.0], 0x90000000);
    game.native_text(batch, &coords, [6.0, y], 0xFFFFFF, true);
    let hw = game.native_text_width(hint);
    game.native_fill(batch, [size[0] - hw - 10.0, y - 3.0, hw + 8.0, 13.0], 0x90000000);
    game.native_text(batch, hint, [size[0] - hw - 6.0, y], 0xC0C0C0, true);
}
