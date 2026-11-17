// A click on the wardrobe: tabs, tiles, colours and buttons.
fn minehouse_mouse(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    use crate::game::minehouse as m;
    if game.native_mod_screen() != Some(m::ID) {
        return;
    }
    *canceled = true;
    if button != 0 {
        return;
    }
    let top_up_w = game.native_text_width("Пополнить");
    let clicked = m::with(|s| {
        let Some(w) = s.wardrobe.clone() else { return Vec::new() };
        let lay = s.ui.layout;
        s.ui.click(&w, &lay, screen.mouse, top_up_w, std::time::Instant::now())
    });
    for c in clicked {
        match c {
            m::Clicked::Send(action) => m::with(|s| s.out.push(action)),
            m::Clicked::OpenStore(url) => {
                crate::game::packs::open_uri(&url);
            }
        }
    }
}
