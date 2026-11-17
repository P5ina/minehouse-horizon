// The button beside the result (`SpeedTradeButton.extractContents`): a button
// with a double trade arrow, greyed while it can't run, and its tooltip.
fn fasttrading_render(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
) {
    use crate::game::fasttrading as m;
    if screen.menu != Some(m::MERCHANT_MENU) || !game.native_on(m::ID) {
        return;
    }
    let Some((seen, running)) = m::with(|s| s.seen.map(|seen| (seen, s.run.is_some()))) else {
        return;
    };
    let why = seen.why();
    let primed = !running && why == m::Why::CanPerform;
    let hover = m::on_button(screen.rect, screen.mouse);
    let (x, y) = (screen.rect[0] + m::BUTTON[0], screen.rect[1] + m::BUTTON[1]);
    let atlas = &game.gui.atlas;
    let sprite = if !primed {
        "gui/sprites/widget/button_disabled"
    } else if hover {
        "gui/sprites/widget/button_highlighted"
    } else {
        "gui/sprites/widget/button"
    };
    batch.sprite_scaled(atlas, sprite, x, y, m::BUTTON[2], m::BUTTON[3], u32::MAX);
    let arrow = if primed {
        "gui/sprites/container/villager/trade_arrow"
    } else {
        "gui/sprites/container/villager/trade_arrow_out_of_stock"
    };
    batch.sprite(atlas, arrow, x + 2.0, y + 5.0);
    batch.sprite(atlas, arrow, x + 8.0, y + 5.0);
    if hover {
        let russian = horizon_client::lang::code().starts_with("ru");
        let lines: Vec<horizon_client::text::Text> = m::tooltip(russian, running, why)
            .into_iter()
            .map(horizon_client::text::Text::literal)
            .collect();
        game.native_gui_tooltip(batch, &lines, screen.mouse);
    }
}
