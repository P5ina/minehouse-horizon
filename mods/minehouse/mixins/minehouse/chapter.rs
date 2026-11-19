// The end of Chapter 1 on the title screen, as the Fabric MineHouse mod does it
// (`ChapterEnd`, minehouse-client 1.17.1): once the server says the chapter is
// over (`minehouse:chapter_end`, sent right before everyone is kicked and the
// server stops), the title is black with the dot the world fell into in place
// of the logo, no splash and no Realms. Singleplayer and Multiplayer stay: you
// can still play, just not on MineHouse until it is back.
//
// The mark is a file (`horizon-mods/.horizon-mod-data/minehouse/chapter-end.txt`,
// the time in milliseconds), so it holds across restarts. It goes when the
// player gets into MineHouse again, at least [`QUIET_MS`] after the end.

use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The server is not back sooner than this: an earlier join is not the next chapter.
pub const QUIET_MS: u64 = 5 * 60 * 1000;
const FILE: &str = "chapter-end.txt";
/// The file is read again at most this often while the title is up.
const RECHECK: Duration = Duration::from_secs(1);

static CACHE: Mutex<Option<(Instant, Option<u64>)>> = Mutex::new(None);
#[cfg(not(test))]
static STARTED: Mutex<Option<Instant>> = Mutex::new(None);

fn path() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap_or_default()
        .join("horizon-mods")
        .join(".horizon-mod-data")
        .join("minehouse")
        .join(FILE)
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// When the chapter ended (ms since the epoch), if it did and MineHouse isn't back yet.
pub fn ended_at() -> Option<u64> {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, value)) = *cache
        && at.elapsed() < RECHECK
    {
        return value;
    }
    let value = std::fs::read_to_string(path()).ok().and_then(|s| parse(&s));
    *cache = Some((Instant::now(), value));
    value
}

pub fn active() -> bool {
    ended_at().is_some()
}

/// `minehouse:chapter_end` came: from now on the title is the black one.
pub fn mark() {
    let p = path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let now = now_ms();
    let _ = std::fs::write(&p, now.to_string());
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), Some(now)));
}

/// MineHouse let the player in again: the next chapter has begun.
pub fn clear() {
    let _ = std::fs::remove_file(path());
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), None));
}

/// A join `now` (ms) ends the black title only after the quiet time.
pub fn over(ended: u64, now: u64) -> bool {
    now >= ended.saturating_add(QUIET_MS)
}

/// Clears the mark when a join to MineHouse (`address`) comes after the quiet time.
pub fn joined(address: Option<&str>) {
    let Some(ended) = ended_at() else { return };
    if is_minehouse(address) && over(ended, now_ms()) {
        clear();
    }
}

pub fn is_minehouse(address: Option<&str>) -> bool {
    address.is_some_and(|a| a.to_ascii_lowercase().contains("minehouse"))
}

pub fn parse(s: &str) -> Option<u64> {
    s.trim().parse().ok()
}

#[cfg(not(test))]
/// Seconds since the black title was first drawn: the glow's breath.
fn seconds() -> f32 {
    let mut started = STARTED.lock().unwrap_or_else(|e| e.into_inner());
    started.get_or_insert_with(Instant::now).elapsed().as_secs_f32()
}

/// Horizontal spans of a disc of radius `r` (GUI pixels) centred on (`cx`, `cy`):
/// one row per pixel, (x, y, width).
pub fn disc(cx: f32, cy: f32, r: f32) -> Vec<(f32, f32, f32)> {
    let mut rows = Vec::new();
    let n = r.ceil() as i32;
    for dy in -n..n {
        let y = dy as f32 + 0.5;
        let half = (r * r - y * y).max(0.0).sqrt();
        if half > 0.0 {
            rows.push((cx - half, cy + dy as f32, half * 2.0));
        }
    }
    rows
}

#[cfg(not(test))]
fn fill_disc(batch: &mut crate::hud::HudBatch, atlas: &crate::hud::HudAtlas, cx: f32, cy: f32, r: f32, color: u32) {
    for (x, y, w) in disc(cx, cy, r) {
        batch.fill(atlas, x, y, w, 1.0, color);
    }
}

pub fn argb(alpha: f32, rgb: u32) -> u32 {
    ((alpha.clamp(0.0, 1.0) * 255.0).round() as u32) << 24 | (rgb & 0x00FF_FFFF)
}

#[cfg(not(test))]
/// Pale blue of the dot's rim and glow (the Fabric mod's `black_dot*.png`).
const GLOW: u32 = 0x9CC8FF;

#[cfg(not(test))]
/// The whole screen black, the dot where the logo was: a speck of black with a
/// pale blue rim, its glow slowly breathing.
pub fn draw(batch: &mut crate::hud::HudBatch, atlas: &crate::hud::HudAtlas, w: f32) {
    let h = batch.height.ceil();
    batch.fill(atlas, 0.0, 0.0, w.ceil(), h, 0xFF00_0000);
    let (cx, cy) = ((w / 2.0).floor(), 54.0);
    let breath = 0.55 + 0.45 * (seconds() * 1.3).sin();
    for (r, a) in [(28.0, 0.05), (22.0, 0.07), (17.0, 0.1), (13.5, 0.16)] {
        fill_disc(batch, atlas, cx, cy, r, argb(a * breath, GLOW));
    }
    fill_disc(batch, atlas, cx, cy, 11.0, argb(0.85, GLOW));
    fill_disc(batch, atlas, cx, cy, 10.0, 0xFF00_0000);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_quiet_time_holds() {
        assert!(!over(1_000, 1_000 + QUIET_MS - 1));
        assert!(over(1_000, 1_000 + QUIET_MS));
    }

    #[test]
    fn only_minehouse_ends_it() {
        assert!(is_minehouse(Some("play.MineHouse.pro")));
        assert!(!is_minehouse(Some("mc.hypixel.net")));
        assert!(!is_minehouse(None));
    }

    #[test]
    fn the_disc_is_round() {
        let rows = disc(0.0, 0.0, 10.0);
        assert_eq!(rows.len(), 20);
        let widest = rows.iter().map(|r| r.2).fold(0.0, f32::max);
        assert!(widest > 19.0 && widest <= 20.0);
        assert!(rows[0].2 < 9.0);
    }

    #[test]
    fn the_glow_fades_with_alpha() {
        assert_eq!(argb(1.0, 0x9CC8FF), 0xFF9C_C8FF);
        assert_eq!(argb(0.0, 0x9CC8FF), 0x009C_C8FF);
    }

    #[test]
    fn the_mark_is_a_number() {
        assert_eq!(parse(" 1795073802000\n"), Some(1_795_073_802_000));
        assert_eq!(parse("x"), None);
    }
}
