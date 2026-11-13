// The pings over the world, as Ping Wheel's `OverlayRenderer` draws them: for
// each, its distance label, its icon (the item, for an item entity) and who
// pinged it; off screen, a direction indicator at the edge of the safe zone.
fn pingwheel_hud(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    gui_hidden: bool,
) {
    use crate::game::pingwheel as m;
    if gui_hidden || !game.native_on(m::ID) || game.native_player().is_none() {
        return;
    }
    let Some(cam) = game.native_camera() else {
        return;
    };
    let size = (batch.width, batch.height);
    let dim = m::dimension_hash(&cam.dimension);
    let items = game.native_client_items();
    let entities = game.native_client_entities();
    // `PingView.update`: an entity ping sits on top of its entity.
    let (pings, c, held) = m::with(|s| {
        for p in &mut s.pings {
            if let Some(id) = p.entity_id {
                let top = entities
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| [e.position[0], e.bounds[1][1], e.position[2]])
                    .or_else(|| items.iter().find(|i| i.id == id).map(|i| [i.position[0], i.position[1] + 0.25, i.position[2]]));
                if let Some(top) = top {
                    p.ping.pos = top;
                }
            }
            p.screen = m::project(p.ping.pos, cam.position, &cam.view, &cam.projection, size);
            p.distance = m::distance(cam.position, p.ping.pos);
        }
        (s.pings.clone(), s.config.clone(), s.player_list_held)
    });
    let atlas = &game.gui.atlas;
    let font_h = 9.0f32;
    let scale_since = |batch: &mut horizon_client::hud::HudBatch, start: usize, at: [f32; 2], s: f32| {
        for v in &mut batch.vertices[start..] {
            v.pos = [at[0] + v.pos[0] * s, at[1] + v.pos[1] * s];
        }
    };
    // `DrawContext.renderPing`: the item, or a 5x5 square turned 45°.
    let draw_icon = |batch: &mut horizon_client::hud::HudBatch, p: &m::Shown, color: u32| {
        let stack = p.entity_id.filter(|_| c.item_icon_visible).and_then(|id| items.iter().find(|i| i.id == id)).map(|i| &i.stack);
        if let Some(stack) = stack {
            let start = batch.vertices.len();
            game.native_gui_draw_item(batch, stack, [-8.0, -8.0]);
            let _ = start;
        } else {
            let r = 2.5 * std::f32::consts::SQRT_2;
            batch.fill_triangle(atlas, [[0.0, -r], [r, 0.0], [0.0, r]], color);
            batch.fill_triangle(atlas, [[0.0, -r], [0.0, r], [-r, 0.0]], color);
        }
    };
    // `DrawContext.renderLabel`: a translucent box and the text, `y_offset` lines from the centre.
    let draw_label = |batch: &mut horizon_client::hud::HudBatch, text: &str, y_offset: f32, color: u32| {
        let w = game.native_text_width(text);
        let (x, y) = (-w * 0.5, -font_h * 0.5 + font_h * y_offset);
        game.native_fill(batch, [x - 2.0, y - 2.0, w + 3.0, font_h + 2.0], m::SHADOW_BLACK);
        game.native_text(batch, text, [x, y], color & 0xFF_FFFF, false);
    };

    let lt = [c.safe_zone[0], c.safe_zone[2]];
    let rb = [size.0 - c.safe_zone[1], size.1 - c.safe_zone[3]];
    let center = [(rb[0] - lt[0]) * 0.5, (rb[1] - lt[1]) * 0.5];
    for p in pings.iter().filter(|p| p.ping.dimension == dim) {
        let Some(([x, y], behind)) = p.screen else { continue };
        if behind && !c.direction_indicator_visible {
            continue;
        }
        let s = p.scale(&c);
        let color = m::WHITE;
        let off_screen = behind || x < 0.0 || y < 0.0 || x > size.0 || y > size.1;
        if c.direction_indicator_visible && off_screen {
            // `DirectionIndicatorRenderer`: the icon and an arrow where the
            // line from the safe zone's centre to the ping leaves it.
            let mut d = [x - lt[0] - center[0], y - lt[1] - center[1]];
            if behind {
                d = [-d[0], -d[1]];
            }
            let angle = d[1].atan2(d[0]);
            let edge = m::angle_rect_intersection(angle, lt, rb);
            let start = batch.vertices.len();
            draw_icon(batch, p, color);
            let back = [(angle + std::f32::consts::PI).cos() * 12.0, (angle + std::f32::consts::PI).sin() * 12.0];
            scale_since(batch, start, [edge[0] + back[0] * s, edge[1] + back[1] * s], s);
            // The arrow: 10 px at a quarter of the ping size, pointing out.
            let a = 10.0 * 0.25 * (c.ping_size as f32 / 100.0) * 2.0;
            let (cos, sin) = (angle.cos(), angle.sin());
            let rot = |px: f32, py: f32| [edge[0] + px * cos - py * sin, edge[1] + px * sin + py * cos];
            batch.fill_triangle(atlas, [rot(a, 0.0), rot(-a * 0.6, -a * 0.7), rot(-a * 0.6, a * 0.7)], color);
        }
        if behind {
            continue;
        }
        // `PingLocationRenderer`.
        let start = batch.vertices.len();
        draw_label(batch, &m::distance_text(p.distance), -1.5, color);
        draw_icon(batch, p, color);
        let verbose = c.player_info_mode == m::PlayerInfoMode::Always
            || (c.player_info_mode == m::PlayerInfoMode::Hold && held);
        if verbose && !p.name.is_empty() {
            draw_label(batch, &p.name, 1.75, m::WHITE);
        }
        scale_since(batch, start, [x, y], s);
    }
}
