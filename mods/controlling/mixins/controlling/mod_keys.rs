// The native mods' keys on Key Binds, above the game's own (as Fabric lists
// mods' key mappings there): a heading per mod, then each key with its button,
// yellow brackets when the key is also taken by the game or another mod.
// Search filters them like the game's rows (name, key, mod). Escape while one
// waits unbinds it, as for the game's keys.
fn controlling_mod_keys(
    value: f32,
    out: &mut Vec<Widget>,
    rows: &mut ListRows,
    ui: &OptionsUi,
    s: &crate::settings::Settings,
    query: &String,
    w: f32,
    cx: f32,
) -> f32 {
    let mut y = value;
    let query = query.to_lowercase();
    let query = query.trim();
    let unbound = "key.keyboard.unknown";
    // Every key in use: the game's, then each mod's (chosen, else default).
    let mut taken: Vec<String> = all_mappings().map(|m| s.keys.get(m.0).to_owned()).collect();
    let mut mods: Vec<(usize, &str, Vec<(usize, &str, String)>)> = Vec::new();
    for (i, m) in s.mods.installed.iter().enumerate() {
        let Some(page) = &m.native else { continue };
        if page.keys.is_empty() || !s.mods.is_on(i) {
            continue;
        }
        let chosen = s.mods.keys.get(&m.id);
        let keys: Vec<(usize, &str, String)> = page
            .keys
            .iter()
            .enumerate()
            .map(|(k, [name, label, default])| {
                let key = chosen.and_then(|c| c.get(name)).unwrap_or(default).clone();
                (k, label.as_str(), key)
            })
            .collect();
        taken.extend(keys.iter().map(|(_, _, key)| key.clone()));
        mods.push((i, m.title.as_str(), keys));
    }
    let row_left = cx - 170.0;
    let scrollbar_x = row_left + 340.0 + 6.0 + 2.0;
    let change_x = scrollbar_x - 50.0 - 10.0 - 5.0 - 75.0;
    for (i, title, keys) in mods {
        let shown: Vec<_> = keys
            .into_iter()
            .filter(|(_, label, key)| {
                query.is_empty()
                    || label.to_lowercase().contains(query)
                    || title.to_lowercase().contains(query)
                    || keybinds::display_name(key).plain().to_lowercase().contains(query)
            })
            .collect();
        if shown.is_empty() {
            continue;
        }
        rows.begin(out, y);
        let heading = Text::literal(title).with_style(&Style {
            bold: Some(true),
            underlined: Some(true),
            ..Style::default()
        });
        out.push(Widget {
            x: 0.0,
            y: y + 7.0,
            w,
            h: 9.0,
            kind: WKind::Text { text: heading, color: u32::MAX, centered: true },
            list: false,
            row: 0,
        });
        y += 20.0;
        rows.end(out, y);
        for (k, label, key) in shown {
            rows.begin(out, y);
            out.push(Widget {
                x: row_left + 2.0,
                y: y + 6.0,
                w: 200.0,
                h: 9.0,
                kind: WKind::Text { text: Text::literal(label), color: u32::MAX, centered: false },
                list: false,
                row: 0,
            });
            let conflict = key != unbound && taken.iter().filter(|t| **t == key).count() > 1;
            let mut message = keybinds::display_name(&key);
            if conflict {
                message = Text::literal("[ ")
                    .append(message.with_color(text::WHITE))
                    .append(" ]")
                    .with_color(text::YELLOW);
            }
            if ui.selected_native_key == Some((i, k)) {
                message = Text::literal("> ")
                    .append(message.with_style(&Style {
                        color: Some(text::WHITE),
                        underlined: Some(true),
                        ..Style::default()
                    }))
                    .append(" <")
                    .with_color(text::YELLOW);
            }
            out.push(button(change_x, y, 75.0, message, Act::NativeKey(i, k)));
            if conflict {
                out.push(Widget {
                    x: change_x - 6.0,
                    y: y + 1.0,
                    w: 3.0,
                    h: 18.0,
                    kind: WKind::Text { text: Text::empty(), color: rgba(255, 255, 0, 255), centered: false },
                    list: false,
                    row: 0,
                });
            }
            y += 20.0;
            rows.end(out, y);
        }
    }
    y
}
