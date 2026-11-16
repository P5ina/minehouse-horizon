// `AnvilScreenMixin.minehouse$realCost`: the cost the anvil screen draws.
fn minehouse_anvil_cost(value: i32) -> i32 {
    use crate::game::minehouse as m;
    m::with(|s| {
        let real = s.anvil.real(s.open_container, value);
        s.drawing_real = real.is_some();
        real.unwrap_or(value)
    })
}
