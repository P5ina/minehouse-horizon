// Each client tick: look at the open merchant menu, start a run the button
// asked for, and take the run's one action (`SpeedTradeButton.tick`).
fn fasttrading_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::fasttrading as m;
    use horizon_client::items::ItemStack;
    use horizon_client::menu::Input;
    let stations = crate::game::stations::INPUT;
    let result_slot = crate::game::stations::RESULT;
    if !game.native_on(m::ID) {
        m::with(|s| *s = m::State::default());
        return;
    }
    let open = game.remote.is_some() && game.remote_menu().1 == m::MERCHANT_MENU;
    let seen = open
        .then(|| {
            let station = game.station.as_ref()?;
            let merchant = station.merchant.as_ref()?;
            let items = game.items();
            let selected = merchant.session.selection_hint;
            let offers = merchant.session.offers.len();
            let Some(offer) = merchant.session.offers.get(selected) else {
                return Some(m::Seen { selected, offers, ..m::Seen::default() });
            };
            let inputs = station.inputs.iter().take(2).flatten();
            let held: Vec<&ItemStack> = game.inventory[..36].iter().flatten().chain(inputs).collect();
            let count = |cost: &horizon_client::merchant::ItemCost| -> u32 {
                held.iter().filter(|s| cost.test(s)).map(|s| s.count).sum()
            };
            let cost = offer.cost_a_count(items);
            let can_pay = count(&offer.base_cost_a) >= cost
                && offer.cost_b.as_ref().is_none_or(|b| count(b) >= b.stack.count);
            // PlayerInventoryUtil.canAcceptStack: an empty slot, or one of the
            // same item with room for the whole result.
            let r = &offer.result;
            let max = items.get(&r.item).map_or(64, |i| i.max_stack);
            let room = game.inventory[..36].iter().any(|s| match s {
                None => true,
                Some(s) => s.item == r.item && s.extra == r.extra && s.damage == r.damage && s.count + r.count <= max,
            });
            let result_ready = station.result.as_ref().is_some_and(|s| s.item == r.item && s.count > 0);
            let blocked = items.get(&r.item).is_some_and(|i| i.max_damage > 0);
            Some(m::Seen {
                selected,
                offers,
                cost,
                out_of_stock: offer.out_of_stock(),
                can_pay,
                room,
                result_ready,
                blocked,
            })
        })
        .flatten();
    let (pressed, run) = m::with(|s| {
        s.seen = seen;
        (s.pressed.take(), s.run)
    });
    let Some(seen) = seen else {
        m::with(|s| s.run = None);
        return;
    };
    let mut run = run;
    if run.is_none()
        && let Some(shift) = pressed
    {
        let why = seen.why();
        if why == m::Why::CanPerform || (why == m::Why::Blocked && shift) {
            run = Some(m::Run::start(&seen));
            game.audio.play("minecraft:ui.button.click", None, 0.25, 1.0);
        }
    }
    let Some(mut r) = run else { return };
    let action = r.step(&seen);
    m::with(|s| s.run = (action != m::Action::Stop).then_some(r));
    match action {
        m::Action::Select(index) => {
            if let Some(merchant) = game.station.as_mut().and_then(|s| s.merchant.as_mut()) {
                merchant.session.selection_hint = index;
            }
            game.send(horizon_protocol::packets::play::Serverbound::SelectTrade(
                horizon_protocol::packets::play::SelectTrade { item: index as i32 },
            ));
        }
        m::Action::TakeResult => {
            game.remote_menu_ops(&[(Some(result_slot), 0, Input::QuickMove)]);
        }
        m::Action::Wait => {}
        m::Action::Stop => {
            // clearSellSlots: the payment goes back to the inventory.
            for i in 0..2 {
                let full = game.station.as_ref().is_some_and(|s| s.inputs.get(i).is_some_and(Option::is_some));
                if full {
                    game.remote_menu_ops(&[(Some(stations + i), 0, Input::QuickMove)]);
                }
            }
        }
    }
}
