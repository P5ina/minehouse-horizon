// `TooltipOverlayHandler.onRenderTooltip`: the food icons a food restores, and
// its saturation, right to left.
fn appleskin_tooltip_draw(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    batch: &mut horizon_client::hud::HudBatch,
    origin: [f32; 2],
) {
    use crate::game::appleskin as m;
    use horizon_client::hud::rgba;
    if !game.native_on(m::ID) {
        return;
    }
    let Some(info) = game.items().get(&stack.item) else { return };
    let Some((nutrition, saturation)) = horizon_client::item_sync::food(stack, info.food) else { return };
    let Some(consumable) = game.consumable(stack) else { return };
    let rotten = consumable.effects.iter().any(|e| {
        matches!(e, horizon_client::items::ConsumeEffect::ApplyEffects { effects, .. } if effects.iter().any(|x| m::harmful(&x.id)))
    });
    let t = m::TooltipFood::new(m::Food { nutrition: nutrition as i32, saturation, rotten, regeneration: None });
    if !t.shown() {
        return;
    }
    let atlas = &game.gui.atlas;
    let hunger = t.food.nutrition;
    let (ox, oy) = (origin[0], origin[1]);
    let mut x = ox + (t.hunger_bars - 1) as f32 * 9.0;
    let mut i = 0;
    while i < t.hunger_bars * 2 {
        batch.sprite_scaled(atlas, "hud/food_empty", x, oy, 9.0, 9.0, u32::MAX);
        let half = hunger - 1 == i;
        // The default value, faint, then the value itself (the same here: nothing modifies it).
        batch.sprite_scaled(atlas, m::food_sprite(rotten, Some(!half)), x, oy, 9.0, 9.0, rgba(255, 255, 255, 64));
        if hunger > i {
            batch.sprite_scaled(atlas, m::food_sprite(rotten, Some(!half)), x, oy, 9.0, 9.0, u32::MAX);
        }
        x -= 9.0;
        i += 2;
    }
    if let Some(text) = &t.hunger_text {
        x += 18.0;
        game.native_text(batch, text, [x + 1.5, oy + 1.5], 0xAA_AAAA, false);
    }
    let y = oy + 10.0;
    let mut x = ox + (t.saturation_bars - 1) as f32 * 7.0;
    for bar in 0..t.saturation_bars {
        let (runs, faded) = t.saturation_icon(bar);
        let alpha = if faded { 0.5 } else { 1.0 };
        for [x0, y0, x1, y1, c] in m::fills(runs, [0, 0], 0, alpha) {
            let c = c as u32;
            game.native_fill(batch, [x + x0 as f32, y + y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32], c);
        }
        x -= 7.0;
    }
    if let Some(text) = &t.saturation_text {
        x += 14.0;
        game.native_text(batch, text, [x + 1.0, y], 0xAA_AAAA, false);
    }
}
