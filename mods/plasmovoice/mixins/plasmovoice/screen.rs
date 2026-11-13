// Draws Plasmo Voice's settings screen: its tabs and the open tab's options.
fn voicechat_screen(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
) {
    use plasmovoice_engine as vc;
    if game.native_mod_screen() != Some(vc::ID) {
        return;
    }
    let hotkeys = vc::HOTKEYS
        .iter()
        .map(|(name, what, default)| (what.to_string(), vc::key_label(&game.native_mod_key(vc::ID, name, default))))
        .collect();
    let measure = |t: &str| game.native_text_width(t);
    for draw in vc::menu_draw([batch.width, batch.height], screen.mouse, &measure, hotkeys) {
        match draw {
            vc::ui::Draw::Fill(rect, argb) => game.native_fill(batch, rect, argb),
            vc::ui::Draw::Text(text, at, rgb, shadow) => game.native_text(batch, &text, at, rgb, shadow),
            vc::ui::Draw::Icon(icon, dst) => {
                if let Some(src) = vc::icons::cell(icon) {
                    game.native_sprite(batch, "horizon/modimage", src, dst);
                }
            }
        }
    }
}
