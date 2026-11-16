// The health and food rows (`HUDOverlayHandler`): Horizon's own rows, with the
// exhaustion under the food bar, the saturation over it, and what the held
// food would restore (hunger, saturation and hearts), flashing.
fn appleskin_hud(
    game: &mut crate::game::Game<'_>,
    inputs: &horizon_client::hud_layout::HealthInputs,
    wi: i32,
    hi: i32,
    draws: &mut Option<Vec<horizon_client::hud_layout::GuiDraw>>,
) {
    use crate::game::appleskin as m;
    use horizon_client::hud_layout::{Blend, GuiDraw, HeartKind};
    if !game.native_on(m::ID) || draws.is_some() {
        return;
    }
    // `FoodHelper.query` + `canConsume`: the main hand's food, else the off hand's.
    let food_of = |game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack| -> Option<(m::Food, bool)> {
        let info = game.items().get(&stack.item)?;
        let (nutrition, saturation) = horizon_client::item_sync::food(stack, info.food)?;
        let consumable = game.consumable(stack)?;
        let (mut rotten, mut regeneration) = (false, None);
        for effect in &consumable.effects {
            if let horizon_client::items::ConsumeEffect::ApplyEffects { effects, .. } = effect {
                for e in effects {
                    rotten |= m::harmful(&e.id);
                    if regeneration.is_none() && e.id.trim_start_matches("minecraft:") == "regeneration" {
                        regeneration = Some((e.amplifier, e.duration));
                    }
                }
            }
        }
        let always = horizon_client::item_sync::can_always_eat(stack, info.can_always_eat);
        Some((m::Food { nutrition: nutrition as i32, saturation, rotten, regeneration }, game.can_eat(stack, always)))
    };
    let main = game.inventory.get(game.selected).cloned().flatten();
    let off = game.offhand.clone();
    let held = [main, off]
        .into_iter()
        .flatten()
        .filter_map(|s| food_of(game, &s))
        .find(|(_, can)| *can)
        .map(|(f, _)| f);

    let mut out = game.native_health_hud_state().layout(inputs, wi, hi);
    let (mut food, mut first_food, mut hearts) = (Vec::new(), None, Vec::new());
    for (k, d) in out.iter().enumerate() {
        if let GuiDraw::Sprite { sprite, x, y, .. } = d {
            if sprite.starts_with("hud/food_empty") {
                first_food.get_or_insert(k);
                food.push([*x, *y]);
            } else if sprite.starts_with("hud/heart/container") {
                hearts.push([*x, *y]);
            }
        }
    }
    // Vanilla draws the hearts from the last to the first.
    hearts.reverse();
    let (exhaustion, saturation, natural, alpha) =
        m::with(|s| (s.exhaustion, s.saturation.unwrap_or(inputs.saturation), s.natural_regeneration, s.flash.alpha));
    if held.is_none() {
        m::with(|s| s.flash.reset());
    }
    let player = m::Player {
        health: inputs.health,
        max_health: inputs.max_health,
        food: inputs.food,
        saturation,
        exhaustion,
        natural_regeneration: natural,
    };
    let mut over = Vec::new();
    if !food.is_empty() {
        m::saturation_overlay(0.0, saturation, &food, 1.0, &mut over);
        if let Some(f) = held {
            m::hunger_overlay(f.nutrition, inputs.food, &food, alpha, f.rotten, &mut over);
            let new_food = inputs.food + f.nutrition;
            let new_saturation = saturation + f.saturation;
            let gained = if new_saturation > new_food as f32 { new_food as f32 - saturation } else { f.saturation };
            m::saturation_overlay(gained, saturation, &food, alpha, &mut over);
        }
    }
    if let Some(f) = held {
        let bad = matches!(inputs.kind, HeartKind::Poisoned | HeartKind::Withered);
        if m::show_estimated_health(&player, false, bad, inputs.regeneration) && hearts.len() <= 1000 {
            let modified = (inputs.health + m::estimated_health(&player, &f)).min(inputs.max_health);
            m::health_overlay(inputs.health, modified, &hearts, alpha, inputs.hardcore, &mut over);
        }
    }
    let to_draws = |o: m::Overlay| -> Vec<GuiDraw> {
        match o {
            m::Overlay::Icon { runs, at, from_x, alpha } => m::fills(runs, at, from_x, alpha)
                .into_iter()
                .map(|[x0, y0, x1, y1, c]| GuiDraw::Fill { x0, y0, x1, y1, color: c as u32 })
                .collect(),
            m::Overlay::Sprite { name, at, color } => vec![GuiDraw::Sprite {
                sprite: std::borrow::Cow::Borrowed(name),
                x: at[0],
                y: at[1],
                w: 9,
                h: 9,
                region: None,
                color,
                blend: Blend::Textured,
            }],
        }
    };
    // The exhaustion goes under the food icons.
    if let Some(k) = first_food
        && let Some(bar) = m::exhaustion_overlay(exhaustion, wi / 2 + 91, hi - 39)
    {
        let under = to_draws(bar);
        out.splice(k..k, under);
    }
    for o in over {
        out.extend(to_draws(o));
    }
    *draws = Some(out);
}
