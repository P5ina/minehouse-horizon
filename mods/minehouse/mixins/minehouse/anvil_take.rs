// `AnvilScreenMixin.minehouse$affordable`: whether the player has the levels
// for the real cost (or infinite materials).
fn minehouse_anvil_take(value: bool, game: &crate::game::Game<'_>) -> bool {
    use crate::game::minehouse as m;
    let Some(slot) = game.station.as_ref().map(|s| s.anvil.cost) else {
        return value;
    };
    match m::with(|s| s.anvil.real(s.open_container, slot)) {
        Some(real) => game.creative || game.experience.level >= real,
        None => value,
    }
}
