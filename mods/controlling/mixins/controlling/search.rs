// Controlling's search (`KeyBindsScreen` filter, `SearchType.NAME`, `KEY`,
// `CATEGORY`): a binding shows when the query is in its name, in its key's label (`G`, `Left Button`), or in its
// category, kept in the game's order: category, then name.
fn controlling_search(
    value: usize,
    mappings: &mut Vec<(&str, &str)>,
    query: &String,
    s: &crate::settings::Settings,
) -> usize {
    use crate::keybinds::{CATEGORIES, all_mappings, display_name};
    use crate::lang;
    let query = query.to_lowercase();
    let query = query.trim();
    if query.is_empty() {
        return value;
    }
    let mut found: Vec<(&str, &str)> = all_mappings()
        .map(|(n, _, c)| (*n, *c))
        .filter(|m| {
            lang::get(m.0).to_lowercase().contains(query)
                || display_name(s.keys.get(m.0)).plain().to_lowercase().contains(query)
                || lang::get(&format!("key.category.minecraft.{}", m.1)).to_lowercase().contains(query)
        })
        .collect();
    found.sort_by(|a, b| {
        let ca = CATEGORIES.iter().position(|c| *c == a.1).unwrap_or(99);
        let cb = CATEGORIES.iter().position(|c| *c == b.1).unwrap_or(99);
        ca.cmp(&cb).then_with(|| lang::get(a.0).cmp(&lang::get(b.0)))
    });
    *mappings = found;
    value
}
