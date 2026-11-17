// The toggle keys, while playing: queued for the next tick, where they act.
fn autoclicker_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::autoclicker as m;
    if !pressed || !game.native_on(m::ID) || !game.native_playing() {
        return;
    }
    for (name, default, toggle) in [
        (m::KEY_LEFT, m::DEFAULT_LEFT, m::Toggle::Left),
        (m::KEY_RIGHT, m::DEFAULT_RIGHT, m::Toggle::Right),
        (m::KEY_HOLD_LEFT, m::UNBOUND, m::Toggle::HoldLeft),
        (m::KEY_HOLD_RIGHT, m::UNBOUND, m::Toggle::HoldRight),
    ] {
        let bound = game.native_mod_key(m::ID, name, default);
        if bound != m::UNBOUND && key == bound {
            m::with(|s| s.queued.push(toggle));
        }
    }
}
