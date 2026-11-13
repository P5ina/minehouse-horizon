//! Plasmo Voice's settings screen (`VoiceSettingsScreen`), laid out as Plasmo
//! lays it out: the title, the tabs down the left with their icons (Devices,
//! Volume, Activation, Overlay, Hotkeys, Advanced, About) and the tab's
//! options on the right, each a label with its control: a toggle, a slider
//! or a drop-down that steps through its values. It lays itself out from a
//! [`View`] each frame and keeps only the open tab, the scroll and what is
//! being dragged.
use crate::Settings;

/// What a hook draws, in GUI pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    /// `[x, y, w, h]` in ARGB.
    Fill([f32; 4], u32),
    /// A line of text at its top left in RGB, with or without a shadow.
    Text(String, [f32; 2], u32, bool),
    /// One of Plasmo's icons ([`crate::icons`]) stretched into `[x, y, w, h]`.
    Icon(&'static str, [f32; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Devices,
    Volume,
    Activation,
    Overlay,
    Hotkeys,
    Advanced,
    About,
}

impl Tab {
    pub const ALL: [Tab; 7] = [Tab::Devices, Tab::Volume, Tab::Activation, Tab::Overlay, Tab::Hotkeys, Tab::Advanced, Tab::About];
    pub fn title(self) -> &'static str {
        ["Devices", "Volume", "Activation", "Overlay", "Hotkeys", "Advanced", "About"][self as usize]
    }
    fn icon(self) -> &'static str {
        ["tabs/devices", "tabs/volume", "tabs/activation", "tabs/overlay", "tabs/hotkeys", "tabs/advanced", "tabs/plasmo"][self as usize]
    }
}

/// What a control changes. Sliders carry the value at the pointer (0..1).
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Tab(Tab),
    Close,
    /// Step a drop-down or toggle; `back` steps backwards.
    Cycle(Opt, bool),
    Slide(Opt, f32),
    /// A player's volume slider and mute, by UUID.
    PlayerSlide(u128, f32),
    PlayerMute(u128),
    /// A source line's volume and mute, by name.
    LineSlide(String, f32),
    LineMute(String),
    Scroll(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opt {
    InputDevice,
    OutputDevice,
    MicVolume,
    Threshold,
    Volume,
    Mode,
    Distance,
    MicMuted,
    VoiceDisabled,
    ActivationIcon,
    IconPosition,
    SourceIcons,
    Overlay,
    OverlayPosition,
    Panning,
}

/// A source line as the Volume tab lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct LineRow {
    pub name: String,
    pub label: String,
    pub icon: &'static str,
    pub volume: u32,
    pub muted: bool,
}

/// A player as the Volume tab lists them.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerRow {
    pub id: u128,
    pub name: String,
    pub volume: u32,
    pub muted: bool,
    pub state: &'static str,
}

/// The voice chat as the screen shows it this frame.
pub struct View {
    pub status: (String, u32),
    pub settings: Settings,
    pub level_db: f32,
    pub connected: bool,
    pub input_devices: Vec<String>,
    pub output_devices: Vec<String>,
    pub input_now: String,
    pub output_now: String,
    pub lines: Vec<LineRow>,
    pub players: Vec<PlayerRow>,
    /// The proximity activation: its name, allowed distances and the one in use.
    pub activation: Option<(String, Vec<i32>, i32)>,
    /// The hotkeys: what each does and the key bound to it, already named.
    pub hotkeys: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    pub rect: [f32; 4],
    pub action: Action,
}

// Vanilla's look: a dark window, stone buttons, white labels.
const SHADE: u32 = 0xA000_0000;
const WINDOW: u32 = 0xF01A_1A1A;
const EDGE: u32 = 0xFF00_0000;
const BUTTON: u32 = 0xFF6F_6F6F;
const BUTTON_HOVER: u32 = 0xFF7E_8AB0;
const BUTTON_EDGE: u32 = 0xFF00_0000;
const BUTTON_LIGHT: u32 = 0xFFA0_A0A0;
const TAB_ON: u32 = 0xFF2C_2C2C;
const SLIDER: u32 = 0xFF3C_3C3C;
const HANDLE: u32 = 0xFFC6_C6C6;
const INK: u32 = 0xFF_FFFF;
const GREY: u32 = 0xA0_A0A0;
const DARK: u32 = 0x60_6060;
const YELLOW: u32 = 0xFF_FF55;
const GREEN: u32 = 0x55_FF55;
const RED: u32 = 0xFF_5555;

const ROW: f32 = 24.0;
const CONTROL_W: f32 = 140.0;

#[derive(Default)]
pub struct Menu {
    pub open: bool,
    close_requested: bool,
    pub tab: Option<Tab>,
    scroll: usize,
    /// The slider being dragged.
    pub dragging: Option<Action>,
    pub message: Option<(String, u32)>,
    widgets: Vec<Widget>,
}

struct Out<'a> {
    draws: Vec<Draw>,
    widgets: Vec<Widget>,
    mouse: [f32; 2],
    measure: &'a dyn Fn(&str) -> f32,
}

fn inside(rect: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= rect[0] && at[0] < rect[0] + rect[2] && at[1] >= rect[1] && at[1] < rect[1] + rect[3]
}

impl Out<'_> {
    fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.draws.push(Draw::Fill(rect, argb));
    }
    fn text(&mut self, text: &str, at: [f32; 2], rgb: u32) {
        self.draws.push(Draw::Text(text.to_owned(), at, rgb, true));
    }
    fn icon(&mut self, icon: &'static str, rect: [f32; 4]) {
        self.draws.push(Draw::Icon(icon, rect));
    }
    fn width(&self, text: &str) -> f32 {
        (self.measure)(text)
    }
    fn fit(&self, text: &str, max: f32) -> String {
        if self.width(text) <= max {
            return text.to_owned();
        }
        let mut cut = text.to_owned();
        while !cut.is_empty() && self.width(&format!("{cut}...")) > max {
            cut.pop();
        }
        format!("{cut}...")
    }
    /// A vanilla button: dark edge, light top-left, stone face.
    fn button(&mut self, rect: [f32; 4], label: &str, action: Action, rgb: u32) {
        let hover = inside(rect, self.mouse);
        let [x, y, w, h] = rect;
        self.fill(rect, BUTTON_EDGE);
        self.fill([x + 1.0, y + 1.0, w - 2.0, h - 2.0], BUTTON_LIGHT);
        self.fill([x + 2.0, y + 2.0, w - 3.0, h - 3.0], if hover { BUTTON_HOVER } else { BUTTON });
        let label = self.fit(label, w - 8.0);
        let tw = self.width(&label);
        self.text(&label, [x + ((w - tw) / 2.0).floor(), y + ((h - 8.0) / 2.0).floor()], if hover { YELLOW } else { rgb });
        self.widgets.push(Widget { rect, action });
    }
    /// A vanilla slider: a dark track with the value on it and a handle at `at` (0..1).
    fn slider(&mut self, rect: [f32; 4], label: &str, at: f32, action: impl Fn(f32) -> Action) {
        let hover = inside(rect, self.mouse);
        let [x, y, w, h] = rect;
        self.fill(rect, BUTTON_EDGE);
        self.fill([x + 1.0, y + 1.0, w - 2.0, h - 2.0], SLIDER);
        let hx = x + 1.0 + (w - 10.0) * at.clamp(0.0, 1.0);
        self.fill([hx, y, 8.0, h], BUTTON_EDGE);
        self.fill([hx + 1.0, y + 1.0, 6.0, h - 2.0], if hover { BUTTON_HOVER } else { HANDLE });
        let tw = self.width(label);
        self.text(label, [x + ((w - tw) / 2.0).floor(), y + ((h - 8.0) / 2.0).floor()], INK);
        // Every pixel across is a widget, so a click or a drag lands on its value.
        let steps = (w as usize).max(1);
        for i in 0..steps {
            let v = i as f32 / (steps - 1).max(1) as f32;
            self.widgets.push(Widget { rect: [x + i as f32, y, 1.0, h], action: action(v) });
        }
    }
    fn title(&mut self, text: &str, x: f32, y: f32) {
        self.text(text, [x, y + 8.0], YELLOW);
    }
    fn label(&mut self, text: &str, x: f32, y: f32, w: f32) {
        let t = self.fit(text, w);
        self.text(&t, [x, y + 6.0], INK);
    }
}

pub fn cycle<T: Clone + PartialEq>(list: &[T], now: &T, back: bool) -> Option<T> {
    if list.is_empty() {
        return None;
    }
    let n = list.len();
    let at = list.iter().position(|x| x == now);
    Some(match (at, back) {
        (Some(i), false) => list[(i + 1) % n].clone(),
        (Some(i), true) => list[(i + n - 1) % n].clone(),
        (None, _) => list[0].clone(),
    })
}

pub use cycle as step;

/// A row of the open tab: its label and how to draw its control.
enum Row {
    Title(String),
    Toggle(String, bool, Opt),
    Drop(String, String, Opt),
    Slide(String, String, f32, Opt),
    Line(LineRow),
    Player(PlayerRow),
    Key(String, String),
    Text(String, u32),
    Meter(f32, f32),
}

impl Menu {
    pub fn open(&mut self, _opened_by: Option<char>) {
        self.open = true;
        self.message = None;
        self.tab.get_or_insert(Tab::Devices);
    }

    pub fn reset(&mut self) {
        self.open = false;
        self.dragging = None;
        self.widgets.clear();
    }

    pub fn close(&mut self) {
        self.reset();
        self.close_requested = true;
    }

    pub fn take_close(&mut self) -> bool {
        std::mem::take(&mut self.close_requested)
    }

    pub fn sync_open(&mut self, ours: bool) {
        if !ours && self.open {
            self.reset();
        }
    }

    pub fn hit(&self, at: [f32; 2]) -> Option<Action> {
        self.widgets.iter().rev().find(|w| inside(w.rect, at)).map(|w| w.action.clone())
    }

    pub fn hit_rect(&self, action: &Action) -> Option<[f32; 4]> {
        self.widgets.iter().find(|w| &w.action == action).map(|w| w.rect)
    }

    pub fn char_typed(&mut self, _c: char) {}

    /// Keys: Escape closes, the arrows scroll.
    pub fn key(&mut self, key: &str, _shift: bool) -> Option<Action> {
        match key {
            "key.keyboard.escape" => Some(Action::Close),
            "key.keyboard.up" => Some(Action::Scroll(1)),
            "key.keyboard.down" => Some(Action::Scroll(-1)),
            _ => None,
        }
    }

    pub fn typing(&self) -> bool {
        false
    }

    pub fn scroll_by(&mut self, steps: i32) {
        self.scroll = (self.scroll as i32 - steps).max(0) as usize;
    }

    /// The menu's own part of an action (tabs, scrolling, closing); the rest
    /// is the voice chat's.
    pub fn act(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::Tab(t) => {
                self.tab = Some(t);
                self.scroll = 0;
                None
            }
            Action::Scroll(d) => {
                self.scroll_by(d);
                None
            }
            Action::Close => {
                self.close();
                None
            }
            other => Some(other),
        }
    }

    pub fn say(&mut self, text: impl Into<String>, error: bool) {
        self.message = Some((text.into(), if error { RED } else { GREEN }));
    }

    fn rows(&self, view: &View) -> Vec<Row> {
        let s = &view.settings;
        let on = |b| b;
        let pct = |v: u32| format!("{v}%");
        match self.tab.unwrap_or(Tab::Devices) {
            Tab::Devices => vec![
                Row::Title("Microphone".into()),
                Row::Drop("Device".into(), if s.input_device.is_empty() { format!("Default ({})", view.input_now) } else { s.input_device.clone() }, Opt::InputDevice),
                Row::Slide("Microphone volume".into(), pct(s.mic_gain), s.mic_gain as f32 / 200.0, Opt::MicVolume),
                Row::Slide("Activation threshold".into(), format!("{} dB", s.threshold), (s.threshold + 60) as f32 / 60.0, Opt::Threshold),
                Row::Meter(view.level_db, s.threshold as f32),
                Row::Title("Speakers".into()),
                Row::Drop("Device".into(), if s.output_device.is_empty() { format!("Default ({})", view.output_now) } else { s.output_device.clone() }, Opt::OutputDevice),
            ],
            Tab::Volume => {
                let mut rows = vec![
                    Row::Title("General".into()),
                    Row::Slide("Volume".into(), pct(s.volume), s.volume as f32 / 200.0, Opt::Volume),
                    Row::Toggle("Microphone muted".into(), on(s.muted), Opt::MicMuted),
                    Row::Toggle("Voice chat disabled".into(), on(s.disabled), Opt::VoiceDisabled),
                ];
                if !view.lines.is_empty() {
                    rows.push(Row::Title("Sources".into()));
                    rows.extend(view.lines.iter().cloned().map(Row::Line));
                }
                rows.push(Row::Title("Players".into()));
                if view.players.is_empty() {
                    rows.push(Row::Text("No players with voice chat".into(), GREY));
                }
                rows.extend(view.players.iter().cloned().map(Row::Player));
                rows
            }
            Tab::Activation => {
                let mut rows = vec![Row::Title("Proximity".into())];
                rows.push(Row::Drop(
                    "Activation".into(),
                    if s.mode == crate::net::Mode::Voice { "Voice".into() } else { "Push to talk".into() },
                    Opt::Mode,
                ));
                match &view.activation {
                    Some((_, distances, now)) if !distances.is_empty() => {
                        let at = distances.iter().position(|d| d == now).unwrap_or(0) as f32 / (distances.len().max(2) - 1) as f32;
                        rows.push(Row::Slide("Distance".into(), format!("{now} blocks"), at, Opt::Distance));
                    }
                    _ => rows.push(Row::Text("The server sets the distance".into(), GREY)),
                }
                rows
            }
            Tab::Overlay => vec![
                Row::Title("Activation icon".into()),
                Row::Toggle("Show activation icon".into(), on(s.show_activation_icon), Opt::ActivationIcon),
                Row::Drop("Position".into(), s.icon_position.label().into(), Opt::IconPosition),
                Row::Title("Source icons".into()),
                Row::Drop("Show source icons".into(), ["When HUD is shown", "Always", "Hidden"][s.source_icons as usize].into(), Opt::SourceIcons),
                Row::Title("Overlay".into()),
                Row::Toggle("Enable overlay".into(), on(s.overlay), Opt::Overlay),
                Row::Drop("Position".into(), s.overlay_position.label().into(), Opt::OverlayPosition),
            ],
            Tab::Hotkeys => {
                let mut rows = vec![Row::Title("General".into())];
                rows.extend(view.hotkeys.iter().map(|(what, key)| Row::Key(what.clone(), key.clone())));
                rows.push(Row::Text("Change them in Options > Mods > Plasmo Voice".into(), GREY));
                rows
            }
            Tab::Advanced => vec![
                Row::Title("Audio".into()),
                Row::Toggle("Panning".into(), on(s.panning), Opt::Panning),
            ],
            Tab::About => vec![
                Row::Title("Plasmo Voice".into()),
                Row::Text("A Horizon client for Plasmo Voice 2.x, by MineHouse".into(), INK),
                Row::Text("Plasmo Voice by Apehum and KPidS (plasmoapp)".into(), GREY),
                Row::Text("Icons: Plasmo Voice, LGPL-3.0".into(), GREY),
                Row::Text("Audio and menus built on Horizon Voice Chat (Nirokx, MIT)".into(), GREY),
            ],
        }
    }

    pub fn draw(&mut self, view: &View, size: [f32; 2], mouse: [f32; 2], measure: &dyn Fn(&str) -> f32) -> Vec<Draw> {
        let mut out = Out { draws: Vec::new(), widgets: Vec::new(), mouse, measure };
        let w = (size[0] - 20.0).min(420.0);
        let h = (size[1] - 20.0).min(280.0);
        let (x0, y0) = (((size[0] - w) / 2.0).floor(), ((size[1] - h) / 2.0).floor());
        out.fill([0.0, 0.0, size[0], size[1]], SHADE);
        out.fill([x0 - 1.0, y0 - 1.0, w + 2.0, h + 2.0], EDGE);
        out.fill([x0, y0, w, h], WINDOW);
        // The title bar: Plasmo Voice and the connection.
        out.icon("microphone", [x0 + 6.0, y0 + 5.0, 12.0, 12.0]);
        out.text("Plasmo Voice", [x0 + 22.0, y0 + 7.0], INK);
        let (status, rgb) = &view.status;
        let tw = out.width("Plasmo Voice") + 30.0;
        let st = out.fit(status, w - tw - 28.0);
        out.text(&st, [x0 + tw, y0 + 7.0], *rgb);
        out.button([x0 + w - 18.0, y0 + 4.0, 14.0, 14.0], "x", Action::Close, INK);
        out.fill([x0, y0 + 21.0, w, 1.0], EDGE);

        // Tabs down the left.
        let tab_w = 92.0;
        let now = self.tab.unwrap_or(Tab::Devices);
        for (i, t) in Tab::ALL.iter().enumerate() {
            let r = [x0 + 4.0, y0 + 26.0 + i as f32 * 22.0, tab_w, 20.0];
            if *t == now {
                out.fill(r, TAB_ON);
                out.fill([r[0], r[1], 2.0, r[3]], YELLOW | 0xFF00_0000);
            } else if inside(r, mouse) {
                out.fill(r, 0xFF24_2424);
            }
            out.icon(t.icon(), [r[0] + 5.0, r[1] + 4.0, 12.0, 12.0]);
            out.text(t.title(), [r[0] + 22.0, r[1] + 6.0], if *t == now { YELLOW } else { INK });
            out.widgets.push(Widget { rect: r, action: Action::Tab(*t) });
        }
        out.fill([x0 + tab_w + 8.0, y0 + 22.0, 1.0, h - 22.0], EDGE);

        // The tab's rows, scrolled.
        let (cx, cw) = (x0 + tab_w + 14.0, w - tab_w - 20.0);
        let (top, bottom) = (y0 + 26.0, y0 + h - 4.0);
        let rows = self.rows(view);
        let fits = (((bottom - top) / ROW).floor() as usize).max(1);
        self.scroll = self.scroll.min(rows.len().saturating_sub(fits));
        let control = |cw: f32| [cx + cw - CONTROL_W, CONTROL_W];
        let mut y = top;
        for row in rows.into_iter().skip(self.scroll).take(fits) {
            let [kx, kw] = control(cw);
            match row {
                Row::Title(t) => out.title(&t, cx, y),
                Row::Toggle(label, on, opt) => {
                    out.label(&label, cx, y, cw - CONTROL_W - 6.0);
                    out.button([kx, y + 1.0, kw, 20.0], if on { "ON" } else { "OFF" }, Action::Cycle(opt, false), if on { GREEN } else { RED });
                }
                Row::Drop(label, value, opt) => {
                    out.label(&label, cx, y, cw - CONTROL_W - 6.0);
                    out.button([kx, y + 1.0, kw, 20.0], &format!("{value} ▼"), Action::Cycle(opt, false), INK);
                }
                Row::Slide(label, value, at, opt) => {
                    out.label(&label, cx, y, cw - CONTROL_W - 6.0);
                    out.slider([kx, y + 1.0, kw, 20.0], &value, at, move |v| Action::Slide(opt, v));
                }
                Row::Line(l) => {
                    out.icon(l.icon, [cx, y + 4.0, 12.0, 12.0]);
                    out.label(&l.label, cx + 16.0, y, cw - CONTROL_W - 22.0);
                    let name = l.name.clone();
                    out.slider([kx, y + 1.0, kw - 22.0, 20.0], &format!("{}%", l.volume), l.volume as f32 / 200.0, move |v| Action::LineSlide(name.clone(), v));
                    out.button([kx + kw - 20.0, y + 1.0, 20.0, 20.0], "", Action::LineMute(l.name.clone()), INK);
                    out.icon(if l.muted { "speaker_disabled" } else { "speaker" }, [kx + kw - 18.0, y + 3.0, 16.0, 16.0]);
                }
                Row::Player(p) => {
                    out.label(&p.name, cx, y, cw - CONTROL_W - 50.0);
                    let sw = out.width(p.state);
                    out.text(p.state, [kx - 6.0 - sw, y + 6.0], DARK);
                    let id = p.id;
                    out.slider([kx, y + 1.0, kw - 22.0, 20.0], &format!("{}%", p.volume), p.volume as f32 / 200.0, move |v| Action::PlayerSlide(id, v));
                    out.button([kx + kw - 20.0, y + 1.0, 20.0, 20.0], "", Action::PlayerMute(p.id), INK);
                    out.icon(if p.muted { "speaker_disabled" } else { "speaker" }, [kx + kw - 18.0, y + 3.0, 16.0, 16.0]);
                }
                Row::Key(what, key) => {
                    out.label(&what, cx, y, cw - CONTROL_W - 6.0);
                    out.fill([kx, y + 1.0, kw, 20.0], BUTTON_EDGE);
                    out.fill([kx + 1.0, y + 2.0, kw - 2.0, 18.0], SLIDER);
                    let tw = out.width(&key);
                    out.text(&key, [kx + ((kw - tw) / 2.0).floor(), y + 7.0], INK);
                }
                Row::Text(t, rgb) => {
                    let t = out.fit(&t, cw);
                    out.text(&t, [cx, y + 6.0], rgb);
                }
                Row::Meter(level, threshold) => {
                    // The live microphone level, green past the threshold.
                    let r = [kx, y + 8.0, kw, 6.0];
                    out.fill(r, BUTTON_EDGE);
                    let f = ((level + 60.0) / 60.0).clamp(0.0, 1.0);
                    out.fill([r[0] + 1.0, r[1] + 1.0, (r[2] - 2.0) * f, r[3] - 2.0], if level > threshold { 0xFF55_DD55 } else { 0xFF88_8888 });
                    let t = ((threshold + 60.0) / 60.0).clamp(0.0, 1.0);
                    out.fill([r[0] + (r[2] - 1.0) * t, r[1] - 2.0, 1.0, r[3] + 4.0], 0xFFFF_FF55);
                    out.label(&format!("Level {level:.0} dB"), cx, y, cw - CONTROL_W - 6.0);
                }
            }
            y += ROW;
        }
        if let Some((m, rgb)) = &self.message {
            let t = out.fit(m, cw);
            out.text(&t, [cx, bottom - 10.0], *rgb);
        }
        self.widgets = out.widgets;
        out.draws
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> View {
        View {
            status: ("Connected".into(), GREEN),
            settings: Settings::default(),
            level_db: -40.0,
            connected: true,
            input_devices: vec!["Mic A".into()],
            output_devices: vec!["Speakers".into()],
            input_now: "Mic A".into(),
            output_now: "Speakers".into(),
            lines: vec![LineRow { name: "proximity".into(), label: "Proximity".into(), icon: "speaker", volume: 100, muted: false }],
            players: vec![PlayerRow { id: 7, name: "Alex".into(), volume: 100, muted: false, state: "voice on" }],
            activation: Some(("proximity".into(), vec![8, 16, 32], 16)),
            hotkeys: vec![("Push to talk".into(), "Left Alt".into())],
        }
    }

    fn texts(draws: &[Draw]) -> Vec<String> {
        draws.iter().filter_map(|d| if let Draw::Text(t, ..) = d { Some(t.clone()) } else { None }).collect()
    }

    #[test]
    fn tabs_switch_and_show_their_options() {
        let mut m = Menu::default();
        m.open(None);
        let measure = |t: &str| t.chars().count() as f32 * 6.0;
        let d = m.draw(&view(), [480.0, 300.0], [0.0, 0.0], &measure);
        let t = texts(&d);
        assert!(t.contains(&"Devices".to_owned()) && t.contains(&"Microphone volume".to_owned()));
        let tab = m.hit_rect(&Action::Tab(Tab::Volume)).unwrap();
        let act = m.hit([tab[0] + 2.0, tab[1] + 2.0]).unwrap();
        assert!(m.act(act).is_none());
        let t = texts(&m.draw(&view(), [480.0, 300.0], [0.0, 0.0], &measure));
        assert!(t.contains(&"Alex".to_owned()) && t.contains(&"Proximity".to_owned()));
        assert!(d.iter().any(|d| matches!(d, Draw::Icon("tabs/devices", _))));
    }

    #[test]
    fn a_click_on_a_slider_sets_the_value_under_the_pointer() {
        let mut m = Menu::default();
        m.open(None);
        let measure = |t: &str| t.chars().count() as f32 * 6.0;
        m.tab = Some(Tab::Volume);
        m.draw(&view(), [480.0, 300.0], [0.0, 0.0], &measure);
        let any = m.widgets.iter().find(|w| matches!(w.action, Action::Slide(Opt::Volume, _))).unwrap().rect;
        let right = m.widgets.iter().filter(|w| matches!(w.action, Action::Slide(Opt::Volume, _))).last().unwrap().rect;
        let left = m.hit([any[0], any[1] + 2.0]).unwrap();
        let end = m.hit([right[0], right[1] + 2.0]).unwrap();
        assert_eq!(left, Action::Slide(Opt::Volume, 0.0));
        assert_eq!(end, Action::Slide(Opt::Volume, 1.0));
    }
}
