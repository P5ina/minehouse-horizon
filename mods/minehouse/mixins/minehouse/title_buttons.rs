// The black title after the end of Chapter 1 (`TitleScreen.createNormalMenuOptions`
// without Realms): Singleplayer and Multiplayer, the icon row and Options/Quit
// moved up into Realms' place. The rest is the title's own `buttons`.
fn minehouse_title_buttons(title: &Title, w: f32, h: f32) -> std::ops::ControlFlow<Vec<Button>> {
    if !matches!(title.mode, Mode::Main)
        || title.rules.borrow().is_some()
        || !crate::title::minehouse_chapter::active()
    {
        return std::ops::ControlFlow::Continue(());
    }
    let (w, h) = (w.ceil(), h.ceil());
    let b = |x: f32, y: f32, w: f32, label: String, enabled: bool, id: u8| Button {
        x,
        y,
        w,
        label,
        enabled,
        id,
        icon: None,
        trailing: None,
        action: None,
    };
    let cx = (w / 2.0).floor();
    let y = (h / 4.0).floor() + 48.0;
    let mut buttons = vec![
        b(cx - 100.0, y, 200.0, lang::get_or("menu.singleplayer", "Singleplayer"), true, 1),
        b(cx - 100.0, y + 24.0, 200.0, lang::get_or("menu.multiplayer", "Multiplayer"), title.allows_multiplayer, 4),
        b(cx - 100.0, y + 72.0, 98.0, lang::get_or("menu.options", "Options..."), true, 3)
            .does(crate::widgets::Action::Options),
        b(cx + 2.0, y + 72.0, 98.0, lang::get_or("menu.quit", "Quit Game"), true, 2).does(crate::widgets::Action::Exit),
    ];
    for (i, (key, plain, id, icon)) in [
        ("gui.friends.open", "Friends", 5, "friends/friends"),
        ("options.language", "Language...", 6, "icon/language"),
        ("options.accessibility", "Accessibility Settings...", 7, "icon/accessibility"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut button = b(
            cx - 34.0 + i as f32 * 24.0,
            y + 48.0,
            20.0,
            lang::get_or(key, plain),
            id != 5 || crate::multiplayer::has_title_entry(),
            id,
        );
        button.icon = Some((icon, 15.0));
        buttons.push(button);
    }
    std::ops::ControlFlow::Break(buttons)
}
