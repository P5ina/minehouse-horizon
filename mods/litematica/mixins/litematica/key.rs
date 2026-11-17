// The printer's and the overlay's toggle keys, while playing.
fn litematica_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use litematica_engine as lm;
    if !pressed || !game.native_on(lm::ID) || !game.native_playing() {
        return;
    }
    let printer = game.native_mod_key(lm::ID, lm::KEY_PRINTER, lm::DEFAULT_PRINTER);
    let render = game.native_mod_key(lm::ID, lm::KEY_RENDER, lm::DEFAULT_RENDER);
    let russian = horizon_client::lang::code().starts_with("ru");
    let line = if printer != lm::UNBOUND && key == printer {
        Some(lm::with(|m| {
            m.printer = !m.printer;
            lm::printer_line(russian, m.printer)
        }))
    } else if render != lm::UNBOUND && key == render {
        Some(lm::with(|m| {
            m.hidden = !m.hidden;
            match (russian, m.hidden) {
                (true, true) => "Схемы скрыты".to_owned(),
                (true, false) => "Схемы видны".to_owned(),
                (false, true) => "Schematics hidden".to_owned(),
                (false, false) => "Schematics shown".to_owned(),
            }
        }))
    } else {
        None
    };
    if let Some(line) = line {
        game.titles.set_overlay(horizon_client::text::Text::literal(line), false);
    }
}
