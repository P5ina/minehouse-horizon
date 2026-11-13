// The pings over the world: a marker where each one is, with who and how far.
fn pingwheel_hud(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    gui_hidden: bool,
) {
    use crate::game::pingwheel as m;
    if gui_hidden || !game.native_on(m::ID) {
        return;
    }
    let Some(cam) = game.native_camera() else {
        return;
    };
    let dim = m::dimension_hash(&cam.dimension);
    let size = game.gui_size();
    let pings = m::with(|s| s.pings.clone());
    for shown in pings.iter().filter(|s| s.ping.dimension == dim) {
        let Some([x, y]) = m::project(shown.ping.pos, cam.position, &cam.view, &cam.projection, size)
        else {
            continue;
        };
        // Fade out over the last second.
        let left = m::PING_DURATION_TICKS.saturating_sub(shown.age).min(20);
        let alpha = (0x40 + left * 0xBF / 20) << 24;
        game.native_fill(batch, [x - 4.0, y - 4.0, 8.0, 8.0], alpha);
        game.native_fill(batch, [x - 3.0, y - 3.0, 6.0, 6.0], alpha | 0x00FF_AA00);
        let text = m::label(&shown.name, m::distance(shown.ping.pos, cam.position));
        let w = game.native_text_width(&text);
        game.native_fill(batch, [x - w / 2.0 - 2.0, y - 17.0, w + 4.0, 10.0], 0x7800_0000);
        game.native_text(batch, &text, [x - w / 2.0, y - 16.0], 0xFF_FFFF, true);
    }
}
