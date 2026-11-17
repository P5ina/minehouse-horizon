// The wardrobe screen (`WardrobeScreen`, without the 3D look at yourself):
// the slot tabs and the shop's tiles on the left; the chosen thing, its
// colours, wear / take off / buy and the balance on the right.
fn minehouse_screen(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
) {
    use crate::game::minehouse as m;
    if game.native_mod_screen() != Some(m::ID) {
        return;
    }
    let mouse = screen.mouse;
    let size = [batch.width, batch.height];
    let Some(w) = m::with(|s| s.wardrobe.clone()) else { return };
    let ui = m::with(|s| s.ui.clone());
    let lay = m::layout(size);

    // A card: background and a 1 px border with the corners cut.
    let card = |batch: &mut horizon_client::hud::HudBatch, r: m::Rect, bg: u32, border: u32| {
        if bg >> 24 != 0 {
            game.native_fill(batch, [r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0], bg);
        }
        if border >> 24 != 0 {
            game.native_fill(batch, [r.x + 2.0, r.y, r.w - 4.0, 1.0], border);
            game.native_fill(batch, [r.x + 2.0, r.y + r.h - 1.0, r.w - 4.0, 1.0], border);
            game.native_fill(batch, [r.x, r.y + 2.0, 1.0, r.h - 4.0], border);
            game.native_fill(batch, [r.x + r.w - 1.0, r.y + 2.0, 1.0, r.h - 4.0], border);
            game.native_fill(batch, [r.x + 1.0, r.y + 1.0, 1.0, 1.0], border);
            game.native_fill(batch, [r.x + r.w - 2.0, r.y + 1.0, 1.0, 1.0], border);
            game.native_fill(batch, [r.x + 1.0, r.y + r.h - 2.0, 1.0, 1.0], border);
            game.native_fill(batch, [r.x + r.w - 2.0, r.y + r.h - 2.0, 1.0, 1.0], border);
        }
    };
    let text = |batch: &mut horizon_client::hud::HudBatch, s: &str, x: f32, y: f32, rgb: u32| {
        game.native_text(batch, s, [x, y], rgb, false);
    };
    let centered = |batch: &mut horizon_client::hud::HudBatch, s: &str, cx: f32, y: f32, rgb: u32| {
        let tw = game.native_text_width(s);
        game.native_text(batch, s, [(cx - tw / 2.0).round(), y], rgb, false);
    };

    game.native_fill(batch, [0.0, 0.0, size[0], size[1]], 0x6000_0000);
    let title = "Гардероб";
    let title_w = game.native_text_width(title);
    game.native_text(batch, title, [(size[0] - title_w) / 2.0, (lay.left.y - 12.0).max(2.0)], m::FG, true);

    // Left: tabs and tiles.
    card(batch, lay.left, m::CARD, m::BORDER);
    let track = m::Rect { x: lay.tabs[0].x, y: lay.tabs[0].y, w: lay.tabs[0].w * 3.0, h: m::TABS_H };
    card(batch, track, m::MUTED_BG, 0);
    for (i, t) in lay.tabs.iter().enumerate() {
        let on = i == ui.slot;
        if on {
            card(batch, *t, 0xFF09_090B, 0xFF27_272A);
        }
        let color = if m::SLOTS[i].1.is_none() {
            m::DISABLED
        } else if on || t.over(mouse) {
            m::FG
        } else {
            m::MUTED
        };
        centered(batch, m::SLOTS[i].0, t.x + t.w / 2.0, t.y + 5.0, color);
    }
    let shown = ui.shown(&w);
    if shown.is_empty() {
        let empty = if ui.slot_id().is_none() { "Скоро" } else { "Пока пусто" };
        centered(batch, empty, lay.left.x + lay.left.w / 2.0, lay.grid.y + 30.0, m::MUTED);
    }
    let mut hovered = None;
    for (i, item) in shown.iter().enumerate() {
        let Some(t) = m::tile(&lay, i, ui.scroll) else { continue };
        let hover = t.over(mouse);
        if hover {
            hovered = Some(item.name.clone());
        }
        let sel = ui.selected.as_deref() == Some(item.id.as_str());
        let border = if sel {
            0xFF00_0000 | m::ACCENT
        } else if w.is_worn(item) {
            0xFF3F_3F46
        } else {
            m::BORDER
        };
        card(batch, t, if hover { m::HOVER_BG } else { m::MUTED_BG }, border);
        game.native_item_model(batch, "minecraft:paper", &item.model(), [t.x + 8.0, t.y + 5.0], 24.0);
        let (tag, color) = if w.is_worn(item) {
            ("Надето".to_string(), m::ACCENT)
        } else if item.owned {
            ("Есть".to_string(), m::GREEN)
        } else {
            (format!("{} MX", item.price), m::MUTED)
        };
        centered(batch, &tag, t.x + t.w / 2.0, t.y + t.h - 10.0, color);
    }
    let hint = match hovered {
        Some(name) => name,
        None if m::max_scroll(&lay, shown.len()) > 0 => "Колесо мыши: ещё".into(),
        None => "Двойной клик: надеть".into(),
    };
    text(batch, &hint, lay.left.x + m::PAD, lay.left.y + lay.left.h - m::PAD - 8.0, m::FAINT);

    // Right: the chosen thing.
    card(batch, lay.right, m::CARD, m::BORDER);
    let item = ui.selected_item(&w);
    let x = lay.right.x + m::PAD;
    let mut y = lay.right.y + m::PAD;
    text(batch, m::SLOTS[ui.slot].0, x, y + 2.0, m::MUTED);
    if let Some(item) = item {
        let worn = w.is_worn(item);
        let badge = if worn {
            "Надето".to_string()
        } else if item.owned {
            "Есть".to_string()
        } else {
            format!("{} MX", item.price)
        };
        let bw = game.native_text_width(&badge) + 10.0;
        let b = m::Rect { x: lay.right.x + lay.right.w - m::PAD - bw, y, w: bw, h: 12.0 };
        card(batch, b, 0, 0xFF00_0000 | if worn { m::ACCENT } else { 0x27272A });
        let color = if worn {
            m::ACCENT
        } else if item.owned {
            m::GREEN
        } else {
            m::FG
        };
        centered(batch, &badge, b.x + b.w / 2.0, y + 2.0, color);
    }
    y += 16.0;
    let lines = match item {
        Some(item) => m::wrap(&item.name, m::RIGHT_W - m::PAD * 2.0, |s| game.native_text_width(s)),
        None => Vec::new(),
    };
    let right = m::right_layout(&lay, item, lines.len());
    match item {
        None => {
            text(batch, "Выбери вещь слева,", x, y + 2.0, m::FG);
            text(batch, "чтобы надеть", x, y + 13.0, m::MUTED);
            text(batch, "или купить", x, y + 24.0, m::MUTED);
        }
        Some(item) => {
            for line in &lines {
                game.native_text(batch, line, [x, y], m::FG, true);
                y += 11.0;
            }
            if let Some(sy) = right.swatch_y {
                text(batch, "Цвет", x, sy - 12.0, m::MUTED);
                let current = ui.color_of(item);
                for i in 0..m::SWATCHES {
                    let Some(s) = right.swatch(&lay, i) else { continue };
                    let rgb = m::swatch_color(item, i) as u32 & 0xFF_FFFF;
                    let on = rgb == current as u32 & 0xFF_FFFF;
                    let hover = s.over(mouse);
                    if on || hover {
                        card(batch, s.grow(2.0), 0, 0xFF00_0000 | if on { m::ACCENT } else { 0x71717A });
                    }
                    card(batch, s, 0xFF00_0000 | rgb, 0x30FF_FFFF);
                    if i == 0 {
                        let ink = if rgb > 0x80_8080 { 0x18181B } else { 0xFFFFFF };
                        centered(batch, "↺", s.x + s.w / 2.0 + 1.0, s.y + 2.0, ink);
                    }
                    if hover {
                        hint_line(game, batch, m::swatch_name(i), mouse);
                    }
                }
            }
        }
    }

    let act = right.action(&lay);
    let enabled = ui.action_enabled(&w);
    let label = m::WardrobeUi::action_label(&w, item);
    if item.is_some_and(|i| w.is_worn(i)) {
        let hover = act.over(mouse);
        card(batch, act, if hover { 0x33F8_7171 } else { 0 }, 0xFF00_0000 | m::RED);
        centered(batch, &label, act.x + act.w / 2.0, act.y + 5.0, m::RED);
    } else {
        let hover = enabled && act.over(mouse);
        let bg = match (enabled, hover) {
            (true, true) => m::PRIMARY_HOVER,
            (true, false) => m::PRIMARY,
            _ => 0xFF27_272A,
        };
        card(batch, act, bg, 0);
        centered(batch, &label, act.x + act.w / 2.0, act.y + 5.0, if enabled { m::ON_PRIMARY } else { m::DISABLED });
    }
    let off = right.take_off(&lay);
    let off_on = !w.worn.is_empty();
    card(batch, off, if off_on && off.over(mouse) { m::HOVER_BG } else { 0 }, m::BORDER);
    centered(batch, "Снять всё", off.x + off.w / 2.0, off.y + 5.0, if off_on { m::FG } else { m::DISABLED });

    // The balance, under a rule.
    game.native_fill(batch, [lay.right.x + 1.0, right.buttons_y + 24.0, lay.right.w - 2.0, 1.0], m::BORDER);
    let by = right.balance_y();
    game.native_text(batch, &format!("{} MX", w.coins), [x, by + 2.0], m::FG, true);
    if w.store.starts_with("https://") {
        let top = "Пополнить";
        let tw = game.native_text_width(top);
        let hover = right.top_up(&lay, tw).over(mouse);
        text(batch, top, lay.right.x + lay.right.w - m::PAD - tw, by + 2.0, if hover { m::FG } else { m::ACCENT });
    }

    // What the hints say along the bottom.
    let hints = "ЛКМ — выбрать   Колесо — листать   Esc — выйти";
    let hw = game.native_text_width(hints);
    let bar = m::Rect { x: ((size[0] - hw) / 2.0 - 8.0).floor(), y: lay.left.y + lay.left.h + 4.0, w: hw + 16.0, h: 14.0 };
    if bar.y + bar.h <= size[1] {
        card(batch, bar, m::CARD, m::BORDER);
        text(batch, hints, bar.x + 8.0, bar.y + 3.0, m::MUTED);
    }

    m::with(|s| {
        s.ui.layout = lay;
        s.ui.right = right;
    });

    // A small tooltip by the pointer.
    fn hint_line(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, s: &str, at: [f32; 2]) {
        let tw = game.native_text_width(s);
        let (x, y) = (at[0] + 8.0, at[1] - 14.0);
        game.native_fill(batch, [x - 3.0, y - 3.0, tw + 6.0, 14.0], 0xF010_0010);
        game.native_text(batch, s, [x, y], 0xFFFFFF, false);
    }
}
