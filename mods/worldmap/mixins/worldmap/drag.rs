// Dragging with the left button moves the map.
fn worldmap_drag(game: &crate::game::Game<'_>, button: i32, delta: [f32; 2], canceled: &mut bool) {
    use crate::game::worldmap as m;
    if game.native_mod_screen() != Some(m::ID) {
        return;
    }
    *canceled = true;
    if button != 0 {
        return;
    }
    m::with(|s| {
        if let Some(v) = s.view.as_mut() {
            let scale = v.scale();
            v.x -= f64::from(delta[0]) / scale;
            v.z -= f64::from(delta[1]) / scale;
        }
    });
}
