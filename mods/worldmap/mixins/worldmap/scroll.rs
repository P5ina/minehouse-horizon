// The wheel zooms about the pointer.
fn worldmap_scroll(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    scroll: [f64; 2],
    canceled: &mut bool,
) {
    use crate::game::worldmap as m;
    if game.native_mod_screen() != Some(m::ID) {
        return;
    }
    *canceled = true;
    let steps = scroll[1].signum() as i32;
    if steps == 0 {
        return;
    }
    let size = game.native_client_state().map_or([1.0, 1.0], |s| s.gui_size);
    m::with(|s| {
        if let Some(v) = s.view.as_mut() {
            v.zoom_about(steps, screen.mouse, size);
        }
    });
}
