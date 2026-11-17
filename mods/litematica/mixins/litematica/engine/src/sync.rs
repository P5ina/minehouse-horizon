//! Syncmatica's client side: the schematics shared on a server (placed there
//! by players with Syncmatica), and downloading one to show it here.
//! Speaks the protocol of Syncmatica 0.3 (sakura-ryoko's port; CC0): every
//! message rides one plugin channel, `syncmatica:main`, as the message's own
//! id (`syncmatica:register_version`...) and then its bytes, written as
//! Minecraft's `FriendlyByteBuf` writes them.
//!
//! The handshake: the server sends its version on join, we answer with ours,
//! it asks for our features and we list them; it then sends every shared
//! placement (`confirm_user`). The features we name decide how the server
//! writes placements to us, so we read them as we declared them.
//! Downloads come in 16 KB pieces, each acknowledged; the file's hash is
//! `UUID.nameUUIDFromBytes(md5(file))`.

pub const CHANNEL: &str = "syncmatica:main";
/// The version we report: the server only refuses "0.0.1".
pub const VERSION: &str = "0.3.20";
/// What we understand, one per line as Syncmatica's `FeatureSet` writes it.
pub const FEATURES: &str = "CORE\nFEATURE\nMODIFY\nMESSAGE\nCORE_EX\nDISPLAY_NAME";
/// The largest download we take.
pub const MAX_FILE: usize = 64 << 20;

/// A schematic shared on the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shared {
    pub id: [u8; 16],
    pub file: String,
    pub hash: [u8; 16],
    pub name: String,
    pub owner: String,
    pub pos: [i32; 3],
    pub dimension: String,
    /// Clockwise quarter turns (`Rotation`'s ordinal: NONE, CLOCKWISE_90,
    /// CLOCKWISE_180, COUNTERCLOCKWISE_90).
    pub rotation: u8,
    /// `Mirror`'s ordinal: NONE, LEFT_RIGHT, FRONT_BACK.
    pub mirror: u8,
}

/// What a message from the server means for the mod.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// The handshake finished: the server's list is in.
    Joined,
    /// A download finished, its hash right.
    Downloaded { id: [u8; 16], bytes: Vec<u8> },
    /// A download failed (cancelled or a wrong hash).
    Failed { id: [u8; 16] },
    /// A placement was moved, turned or mirrored on the server.
    Moved { id: [u8; 16] },
    /// A placement was removed on the server.
    Removed { id: [u8; 16] },
    /// A placement was shared.
    Added { id: [u8; 16] },
    /// A message from the server (a translation key, mostly).
    Message(String),
}

struct Download {
    id: [u8; 16],
    bytes: Vec<u8>,
}

/// The state of Syncmatica on the server joined.
#[derive(Default)]
pub struct Sync {
    /// The server speaks Syncmatica and the handshake finished.
    pub joined: bool,
    pub shared: Vec<Shared>,
    download: Option<Download>,
    /// Payloads for `CHANNEL`, to send.
    pub out: Vec<Vec<u8>>,
}

impl Sync {
    pub fn get(&self, id: [u8; 16]) -> Option<&Shared> {
        self.shared.iter().find(|s| s.id == id)
    }

    /// The placement being downloaded and the bytes so far.
    pub fn downloading(&self) -> Option<([u8; 16], usize)> {
        self.download.as_ref().map(|d| (d.id, d.bytes.len()))
    }

    /// Asks for a shared placement's file.
    pub fn request(&mut self, id: [u8; 16]) {
        if let Some(d) = self.download.take() {
            self.send("cancel_litematic", |w| w.uuid(d.id));
        }
        self.download = Some(Download { id, bytes: Vec::new() });
        self.send("request_download", |w| w.uuid(id));
    }

    fn send(&mut self, kind: &str, body: impl FnOnce(&mut Writer)) {
        let mut w = Writer::default();
        w.utf(&format!("syncmatica:{kind}"));
        body(&mut w);
        self.out.push(w.0);
    }

    /// One payload on `CHANNEL` from the server.
    pub fn receive(&mut self, data: &[u8]) -> Result<Vec<Event>, String> {
        let mut r = Reader::new(data);
        let kind = r.utf()?;
        let Some(kind) = kind.strip_prefix("syncmatica:") else {
            return Ok(Vec::new());
        };
        let mut events = Vec::new();
        match kind {
            // A fresh join: anything from the last server goes.
            "register_version" => {
                let _server = r.utf()?;
                *self = Sync::default();
                self.send("register_version", |w| w.utf(VERSION));
            }
            "feature_request" => self.send("feature", |w| w.utf(FEATURES)),
            // The server's own features: it writes to us by ours.
            "feature" => {}
            "confirm_user" => {
                let n = r.int()?;
                if !(0..=100_000).contains(&n) {
                    return Err(format!("{n} placements?"));
                }
                self.shared.clear();
                for _ in 0..n {
                    self.shared.push(read_placement(&mut r)?);
                }
                self.joined = true;
                events.push(Event::Joined);
            }
            "register_metadata" => {
                let p = read_placement(&mut r)?;
                let id = p.id;
                self.shared.retain(|s| s.id != id);
                self.shared.push(p);
                events.push(Event::Added { id });
            }
            "remove_syncmatic" => {
                let id = r.uuid()?;
                self.shared.retain(|s| s.id != id);
                events.push(Event::Removed { id });
            }
            "modify" => {
                let id = r.uuid()?;
                let (pos, dimension, rotation, mirror) = read_position(&mut r)?;
                if let Some(s) = self.shared.iter_mut().find(|s| s.id == id) {
                    s.pos = pos;
                    s.dimension = dimension;
                    s.rotation = rotation;
                    s.mirror = mirror;
                    events.push(Event::Moved { id });
                }
            }
            "mesage" => {
                let _kind = r.utf()?;
                events.push(Event::Message(r.utf()?));
            }
            "send_litematic" => {
                let id = r.uuid()?;
                let size = r.int()?;
                let chunk = r.bytes(usize::try_from(size).map_err(|_| "a negative size")?)?;
                match self.download.as_mut() {
                    Some(d) if d.id == id && d.bytes.len() + chunk.len() <= MAX_FILE => {
                        d.bytes.extend_from_slice(chunk);
                        self.send("received_litematic", |w| w.uuid(id));
                    }
                    Some(d) if d.id == id => {
                        self.download = None;
                        self.send("cancel_litematic", |w| w.uuid(id));
                        events.push(Event::Failed { id });
                    }
                    // Not ours (any more).
                    _ => self.send("cancel_litematic", |w| w.uuid(id)),
                }
            }
            "finished_litematic" => {
                let id = r.uuid()?;
                if self.download.as_ref().is_some_and(|d| d.id == id) {
                    let d = self.download.take().unwrap_or_else(|| unreachable!());
                    let right = self.get(id).is_some_and(|s| s.hash == file_hash(&d.bytes));
                    events.push(if right { Event::Downloaded { id, bytes: d.bytes } } else { Event::Failed { id } });
                }
            }
            "cancel_litematic" => {
                let id = r.uuid()?;
                if self.download.as_ref().is_some_and(|d| d.id == id) {
                    self.download = None;
                    events.push(Event::Failed { id });
                }
            }
            // Sharing and moving from here aren't offered.
            _ => {}
        }
        Ok(events)
    }
}

fn read_position(r: &mut Reader<'_>) -> Result<([i32; 3], String, u8, u8), String> {
    let pos = r.block_pos()?;
    let dimension = r.utf()?;
    let rotation = r.int()?;
    let mirror = r.int()?;
    if !(0..4).contains(&rotation) || !(0..3).contains(&mirror) {
        return Err(format!("rotation {rotation}, mirror {mirror}?"));
    }
    // Sub-regions moved apart (CORE_EX): not followed, read past.
    let n = r.int()?;
    for _ in 0..n.clamp(0, 10_000) {
        r.utf()?;
        r.block_pos()?;
        r.int()?;
        r.int()?;
    }
    Ok((pos, dimension, rotation as u8, mirror as u8))
}

fn read_placement(r: &mut Reader<'_>) -> Result<Shared, String> {
    let id = r.uuid()?;
    let file = r.utf()?;
    let hash = r.uuid()?;
    let name = r.utf()?; // DISPLAY_NAME
    let _owner_id = r.uuid()?; // CORE_EX
    let owner = r.utf()?;
    let _modified_id = r.uuid()?;
    let _modified = r.utf()?;
    let (pos, dimension, rotation, mirror) = read_position(r)?;
    Ok(Shared { id, file, hash, name, owner, pos, dimension, rotation, mirror })
}

/// `FriendlyByteBuf`'s reads.
pub struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}
impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Self { b, i: 0 }
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.i.checked_add(n).filter(|&e| e <= self.b.len()).ok_or("the message ends early")?;
        let s = &self.b[self.i..end];
        self.i = end;
        Ok(s)
    }
    fn var_int(&mut self) -> Result<i32, String> {
        let mut v: u32 = 0;
        for k in 0..5 {
            let b = self.bytes(1)?[0];
            v |= u32::from(b & 0x7f) << (7 * k);
            if b & 0x80 == 0 {
                return Ok(v as i32);
            }
        }
        Err("a VarInt too long".into())
    }
    fn utf(&mut self) -> Result<String, String> {
        let n = self.var_int()?;
        let n = usize::try_from(n).map_err(|_| "a negative length")?;
        Ok(String::from_utf8_lossy(self.bytes(n)?).into_owned())
    }
    fn int(&mut self) -> Result<i32, String> {
        let b = self.bytes(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn long(&mut self) -> Result<i64, String> {
        let b = self.bytes(8)?;
        let mut a = [0; 8];
        a.copy_from_slice(b);
        Ok(i64::from_be_bytes(a))
    }
    fn uuid(&mut self) -> Result<[u8; 16], String> {
        let mut a = [0; 16];
        a.copy_from_slice(self.bytes(16)?);
        Ok(a)
    }
    /// `BlockPos.asLong`: x 26 bits, z 26 bits, y 12 bits.
    fn block_pos(&mut self) -> Result<[i32; 3], String> {
        let v = self.long()?;
        Ok([(v >> 38) as i32, ((v << 52) >> 52) as i32, ((v << 26) >> 38) as i32])
    }
}

/// `FriendlyByteBuf`'s writes.
#[derive(Default)]
pub struct Writer(pub Vec<u8>);
impl Writer {
    pub fn var_int(&mut self, v: i32) {
        let mut v = v as u32;
        loop {
            if v & !0x7f == 0 {
                self.0.push(v as u8);
                return;
            }
            self.0.push((v & 0x7f | 0x80) as u8);
            v >>= 7;
        }
    }
    pub fn utf(&mut self, s: &str) {
        self.var_int(s.len() as i32);
        self.0.extend_from_slice(s.as_bytes());
    }
    pub fn uuid(&mut self, id: [u8; 16]) {
        self.0.extend_from_slice(&id);
    }
    pub fn int(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    pub fn block_pos(&mut self, p: [i32; 3]) {
        let v = (i64::from(p[0]) & 0x3ff_ffff) << 38 | (i64::from(p[2]) & 0x3ff_ffff) << 12 | (i64::from(p[1]) & 0xfff);
        self.0.extend_from_slice(&v.to_be_bytes());
    }
}

/// A UUID as Java prints it.
pub fn uuid_text(id: [u8; 16]) -> String {
    let h: String = id.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// Syncmatica's hash of a file: `UUID.nameUUIDFromBytes(md5(file))`.
pub fn file_hash(bytes: &[u8]) -> [u8; 16] {
    let mut h = md5(&md5(bytes));
    h[6] = h[6] & 0x0f | 0x30;
    h[8] = h[8] & 0x3f | 0x80;
    h
}

/// MD5 (RFC 1321).
pub fn md5(data: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11,
        16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32).collect();
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_le_bytes());
    let (mut a0, mut b0, mut c0, mut d0) = (0x6745_2301u32, 0xefcd_ab89u32, 0x98ba_dcfeu32, 0x1032_5476u32);
    for block in msg.chunks(64) {
        let m: Vec<u32> = block.chunks(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0; 16];
    for (i, v) in [a0, b0, c0, d0].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn message(kind: &str, body: impl FnOnce(&mut Writer)) -> Vec<u8> {
        let mut w = Writer::default();
        w.utf(&format!("syncmatica:{kind}"));
        body(&mut w);
        w.0
    }

    fn kind(out: &[u8]) -> String {
        Reader::new(out).utf().unwrap()
    }

    #[test]
    fn md5_and_the_file_hash() {
        assert_eq!(hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(&md5(b"The quick brown fox jumps over the lazy dog")), "9e107d9d372bb6826bd81d3542a419d6");
        let long = vec![b'a'; 1000];
        assert_eq!(hex(&md5(&long)), "cabe45dcc9ae5b66ba86600cca6b8ba8");
        // Java: UUID.nameUUIDFromBytes(md5("")) — the v3 UUID of those 16 bytes.
        let h = file_hash(b"");
        assert_eq!(h[6] >> 4, 3);
        assert_eq!(h[8] >> 6, 2);
    }

    #[test]
    fn block_positions_round_trip() {
        for p in [[0, 0, 0], [-1, -64, -1], [123_456, 319, -654_321], [-30_000_000, -2048, 29_999_999]] {
            let mut w = Writer::default();
            w.block_pos(p);
            assert_eq!(Reader::new(&w.0).block_pos().unwrap(), p);
        }
    }

    #[test]
    fn the_handshake_the_list_and_a_download() {
        let mut s = Sync::default();
        s.receive(&message("register_version", |w| w.utf("0.3.20"))).unwrap();
        assert_eq!(kind(&s.out.remove(0)), "syncmatica:register_version");
        s.receive(&message("feature_request", |_| {})).unwrap();
        let reply = s.out.remove(0);
        let mut r = Reader::new(&reply);
        assert_eq!(r.utf().unwrap(), "syncmatica:feature");
        assert_eq!(r.utf().unwrap(), FEATURES);

        let file = b"not really a schematic".to_vec();
        let id = [7; 16];
        let ev = s
            .receive(&message("confirm_user", |w| {
                w.int(1);
                w.uuid(id);
                w.utf("house.litematic");
                w.uuid(file_hash(&file));
                w.utf("House");
                w.uuid([1; 16]);
                w.utf("Steve");
                w.uuid([1; 16]);
                w.utf("Steve");
                w.block_pos([10, 64, -20]);
                w.utf("minecraft:overworld");
                w.int(1);
                w.int(2);
                w.int(0);
            }))
            .unwrap();
        assert_eq!(ev, vec![Event::Joined]);
        assert!(s.joined);
        let p = &s.shared[0];
        assert_eq!((p.name.as_str(), p.owner.as_str(), p.pos, p.rotation, p.mirror), ("House", "Steve", [10, 64, -20], 1, 2));

        s.request(id);
        assert_eq!(kind(&s.out.remove(0)), "syncmatica:request_download");
        for part in file.chunks(10) {
            let ev = s
                .receive(&message("send_litematic", |w| {
                    w.uuid(id);
                    w.int(part.len() as i32);
                    w.0.extend_from_slice(part);
                }))
                .unwrap();
            assert!(ev.is_empty());
            assert_eq!(kind(&s.out.remove(0)), "syncmatica:received_litematic");
        }
        let ev = s.receive(&message("finished_litematic", |w| w.uuid(id))).unwrap();
        assert_eq!(ev, vec![Event::Downloaded { id, bytes: file }]);

        // Moved on the server.
        let ev = s
            .receive(&message("modify", |w| {
                w.uuid(id);
                w.block_pos([0, 70, 0]);
                w.utf("minecraft:the_nether");
                w.int(3);
                w.int(0);
                w.int(0);
                w.uuid([2; 16]);
                w.utf("Alex");
            }))
            .unwrap();
        assert_eq!(ev, vec![Event::Moved { id }]);
        assert_eq!((s.shared[0].pos, s.shared[0].rotation), ([0, 70, 0], 3));
        let ev = s.receive(&message("remove_syncmatic", |w| w.uuid(id))).unwrap();
        assert_eq!(ev, vec![Event::Removed { id }]);
        assert!(s.shared.is_empty());
    }

    #[test]
    fn a_wrong_hash_fails() {
        let mut s = Sync::default();
        s.shared.push(Shared {
            id: [3; 16],
            file: "a".into(),
            hash: [0; 16],
            name: "a".into(),
            owner: String::new(),
            pos: [0; 3],
            dimension: String::new(),
            rotation: 0,
            mirror: 0,
        });
        s.request([3; 16]);
        s.receive(&message("send_litematic", |w| {
            w.uuid([3; 16]);
            w.int(2);
            w.0.extend_from_slice(b"hi");
        }))
        .unwrap();
        let ev = s.receive(&message("finished_litematic", |w| w.uuid([3; 16]))).unwrap();
        assert_eq!(ev, vec![Event::Failed { id: [3; 16] }]);
    }
}
