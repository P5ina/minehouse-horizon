// A press on the button (`SpeedTradeButton.onPress`): the screen hooks only
// read, so the next tick starts the run. Shift trades a damageable result too.
fn fasttrading_press(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    button: i32,
    modifiers: u32,
    canceled: &mut bool,
) {
    use crate::game::fasttrading as m;
    if button != 0 || screen.menu != Some(m::MERCHANT_MENU) || !game.native_on(m::ID) {
        return;
    }
    if m::on_button(screen.rect, screen.mouse) {
        m::with(|s| s.pressed = Some(modifiers & 1 != 0));
        *canceled = true;
    }
}
