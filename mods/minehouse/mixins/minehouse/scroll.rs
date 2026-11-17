// The wheel over the wardrobe scrolls the tiles a row at a time.
fn minehouse_scroll(
    game: &crate::game::Game<'_>,
    scroll: [f64; 2],
    canceled: &mut bool,
) {
    use crate::game::minehouse as m;
    if game.native_mod_screen() != Some(m::ID) {
        return;
    }
    *canceled = true;
    let rows = if scroll[1] > 0.0 { -1 } else if scroll[1] < 0.0 { 1 } else { 0 };
    m::with(|s| {
        let Some(w) = s.wardrobe.clone() else { return };
        let lay = s.ui.layout;
        s.ui.scroll_by(&w, &lay, rows);
    });
}
