// Chat Heads for Horizon: a port of Chat Heads 1.3 for Fabric
// (https://github.com/dzwdz/chat_heads, MIT): the head of whoever sent a chat
// line in front of it. A player's signed message names its sender; a system
// line (a server's own chat format, a join message) is matched against the
// names in the player list, as Chat Heads does.
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game.

pub const ID: &str = "chatheads";
pub const NAME: &str = "Chat Heads";
pub const VERSION: &str = "1.3.2";
pub const DESCRIPTION: &str = "Shows player heads next to their chat messages.";

fn name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The player of `players` (name, uuid) whose name appears first in `line` as
/// a whole word (not part of a longer name), the longer name winning at the
/// same place (`ChatHeads.getChatOwner`'s search of the message's text).
pub fn owner_in(line: &str, players: &[(String, u128)]) -> Option<u128> {
    let mut best: Option<(usize, usize, u128)> = None;
    for (name, uuid) in players {
        if name.is_empty() {
            continue;
        }
        let mut from = 0;
        while let Some(i) = line[from..].find(name.as_str()) {
            let at = from + i;
            let end = at + name.len();
            let before = line[..at].chars().next_back().is_none_or(|c| !name_char(c));
            let after = line[end..].chars().next().is_none_or(|c| !name_char(c));
            if before && after {
                let better = match best {
                    None => true,
                    Some((b_at, b_len, _)) => at < b_at || (at == b_at && name.len() > b_len),
                };
                if better {
                    best = Some((at, name.len(), *uuid));
                }
                break;
            }
            from = at + name[..].chars().next().map_or(1, char::len_utf8);
        }
    }
    best.map(|(_, _, uuid)| uuid)
}

/// A profile for a `minecraft:object` player sprite: its id (as vanilla
/// writes UUIDs) and name.
pub fn profile_json(uuid: u128, name: &str) -> String {
    let h = format!("{uuid:032x}");
    format!(
        r#"{{"id":"{}-{}-{}-{}-{}","name":"{}"}}"#,
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32],
        name.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players() -> Vec<(String, u128)> {
        vec![("Timur".into(), 1), ("Tim".into(), 2), ("_Alekseyka".into(), 3), ("Bob".into(), 4)]
    }

    #[test]
    fn finds_the_sender() {
        assert_eq!(owner_in("[Игрок] Timur: привет", &players()), Some(1));
        assert_eq!(owner_in("<Tim> hi Timur", &players()), Some(2));
        assert_eq!(owner_in("_Alekseyka joined the game", &players()), Some(3));
        // A name inside a longer word is not a match.
        assert_eq!(owner_in("Bobby says hi", &players()), None);
        assert_eq!(owner_in("Timurka: hi, Bob", &players()), Some(4));
        assert_eq!(owner_in("nobody here", &players()), None);
    }

    #[test]
    fn profiles() {
        assert_eq!(
            profile_json(0x0123456789abcdef0123456789abcdef, "Ti\"m"),
            r#"{"id":"01234567-89ab-cdef-0123-456789abcdef","name":"Ti\"m"}"#
        );
    }
}
