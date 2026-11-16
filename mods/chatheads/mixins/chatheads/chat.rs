// A chat line about to show: put its sender's head in front of it.
fn chatheads_chat(
    game: &mut crate::game::Game<'_>,
    overlay: bool,
    sender: u128,
    message: &mut horizon_client::text::Text,
) {
    use crate::game::chatheads as m;
    use horizon_client::text::{InlineObject, Text};
    if overlay || !game.native_on(m::ID) {
        return;
    }
    let Some((players, _, _)) = game.remote_tab_players() else {
        return;
    };
    let players: Vec<(String, u128)> = players.into_iter().map(|p| (p.name, p.uuid)).collect();
    let owner = if sender != 0 && players.iter().any(|p| p.1 == sender) {
        Some(sender)
    } else {
        m::owner_in(&message.plain(), &players)
    };
    let Some((name, uuid)) = owner.and_then(|o| players.into_iter().find(|p| p.1 == o)) else {
        return;
    };
    let Ok(profile) = serde_json::from_str(&m::profile_json(uuid, &name)) else {
        return;
    };
    let mut head = Text::literal("\u{FFFC}");
    head.spans[0].style.inline_object = Some(InlineObject::Player { profile, hat: true, fallback: String::new() });
    *message = head.append(" ").append(std::mem::take(message));
}
