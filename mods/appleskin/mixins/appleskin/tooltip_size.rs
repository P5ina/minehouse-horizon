// `TooltipOverlayHandler.onItemTooltip`: room for the food values under a food's tooltip.
fn appleskin_tooltip_size(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    size: &mut Option<[f32; 2]>,
) {
    use crate::game::appleskin as m;
    if !game.native_on(m::ID) || size.is_some() {
        return;
    }
    let Some(info) = game.items().get(&stack.item) else { return };
    let Some((nutrition, saturation)) = horizon_client::item_sync::food(stack, info.food) else { return };
    if game.consumable(stack).is_none() {
        return;
    }
    let t = m::TooltipFood::new(m::Food { nutrition: nutrition as i32, saturation, rotten: false, regeneration: None });
    if t.shown() {
        *size = Some([t.width(|s| game.mh_text_width(s)), m::TooltipFood::HEIGHT]);
    }
}
