//! Plasmo Voice's encryption: the client makes an RSA key pair and sends the
//! public half (X.509 SubjectPublicKeyInfo, as Java's `getEncoded`); the server
//! seals its AES key with it (Java's `Cipher.getInstance("RSA")`: PKCS#1 v1.5);
//! voice frames are then `AES/CBC/PKCS5Padding`, a random 16-byte IV first.
//! Horizon's lock file has no RSA crate, so the RSA here is num-bigint's
//! arithmetic over randomness from ring.
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use num_bigint::BigUint;
use ring::rand::{SecureRandom, SystemRandom};

pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    SystemRandom::new().fill(&mut b).expect("system randomness");
    b
}

// --- RSA ------------------------------------------------------------------------------

pub struct RsaKey {
    pub n: BigUint,
    pub e: BigUint,
    d: BigUint,
}

const SMALL_PRIMES: [u32; 54] = [
    3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101,
    103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193, 197,
    199, 211, 223, 227, 229, 233, 239, 241, 251, 257,
];

fn random_below(limit: &BigUint) -> BigUint {
    let bytes = (limit.bits() as usize).div_ceil(8) + 8;
    BigUint::from_bytes_be(&random_bytes(bytes)) % limit
}

fn probably_prime(n: &BigUint, rounds: usize) -> bool {
    let one = BigUint::from(1u32);
    let two = BigUint::from(2u32);
    if *n < two {
        return false;
    }
    for &p in &SMALL_PRIMES {
        let p = BigUint::from(p);
        if *n == p {
            return true;
        }
        if (n % &p).bits() == 0 {
            return false;
        }
    }
    let n1 = n - &one;
    let s = n1.trailing_zeros().unwrap_or(0);
    let d = &n1 >> s;
    'witness: for _ in 0..rounds {
        let a = random_below(&(n - 3u32)) + &two;
        let mut x = a.modpow(&d, n);
        if x == one || x == n1 {
            continue;
        }
        for _ in 1..s {
            x = x.modpow(&two, n);
            if x == n1 {
                continue 'witness;
            }
        }
        return false;
    }
    true
}

fn random_prime(bits: u64) -> BigUint {
    loop {
        let mut b = random_bytes(bits.div_ceil(8) as usize);
        b[0] |= 0xC0; // the top two bits: p*q has all the bits
        let last = b.len() - 1;
        b[last] |= 1;
        let candidate = BigUint::from_bytes_be(&b);
        if probably_prime(&candidate, 24) {
            return candidate;
        }
    }
}

impl RsaKey {
    /// A fresh key of `bits` (2048 as Plasmo's own client); takes a moment, so
    /// the caller runs it off the game thread.
    pub fn generate(bits: u64) -> Self {
        let e = BigUint::from(65_537u32);
        loop {
            let p = random_prime(bits / 2);
            let q = random_prime(bits / 2);
            if p == q {
                continue;
            }
            let n = &p * &q;
            let phi = (&p - 1u32) * (&q - 1u32);
            if let Some(d) = e.modinv(&phi) {
                return Self { n, e, d };
            }
        }
    }

    fn size(&self) -> usize {
        (self.n.bits() as usize).div_ceil(8)
    }

    /// PKCS#1 v1.5 decryption (block type 2).
    pub fn decrypt(&self, sealed: &[u8]) -> Option<Vec<u8>> {
        let k = self.size();
        if sealed.len() != k {
            return None;
        }
        let m = BigUint::from_bytes_be(sealed).modpow(&self.d, &self.n);
        let mut em = m.to_bytes_be();
        while em.len() < k {
            em.insert(0, 0);
        }
        if em[0] != 0 || em[1] != 2 {
            return None;
        }
        let sep = em[2..].iter().position(|&b| b == 0)? + 2;
        if sep < 10 {
            return None;
        }
        Some(em[sep + 1..].to_vec())
    }

    /// PKCS#1 v1.5 encryption with the public half, as the server does (tests).
    pub fn encrypt(&self, plain: &[u8]) -> Vec<u8> {
        let k = self.size();
        let mut em = vec![0u8, 2];
        let pad = k - 3 - plain.len();
        em.extend(random_bytes(pad * 2).into_iter().filter(|&b| b != 0).take(pad));
        while em.len() < 2 + pad {
            em.push(0x55);
        }
        em.push(0);
        em.extend_from_slice(plain);
        let mut c = BigUint::from_bytes_be(&em).modpow(&self.e, &self.n).to_bytes_be();
        while c.len() < k {
            c.insert(0, 0);
        }
        c
    }

    /// The public key as X.509 SubjectPublicKeyInfo DER: what Java's
    /// `X509EncodedKeySpec` on the server reads.
    pub fn public_der(&self) -> Vec<u8> {
        let rsa_key = der_seq(&[der_uint(&self.n), der_uint(&self.e)]);
        // AlgorithmIdentifier: rsaEncryption (1.2.840.113549.1.1.1), NULL.
        let algorithm = der_seq(&[
            vec![0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01],
            vec![0x05, 0x00],
        ]);
        let mut bits = vec![0u8];
        bits.extend(rsa_key);
        der_seq(&[algorithm, der(0x03, &bits)])
    }
}

fn der_len(n: usize) -> Vec<u8> {
    if n < 0x80 {
        return vec![n as u8];
    }
    let b = (n as u32).to_be_bytes();
    let skip = b.iter().take_while(|&&x| x == 0).count();
    let mut out = vec![0x80 | (4 - skip) as u8];
    out.extend_from_slice(&b[skip..]);
    out
}

fn der(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend(der_len(body.len()));
    out.extend_from_slice(body);
    out
}

fn der_seq(parts: &[Vec<u8>]) -> Vec<u8> {
    der(0x30, &parts.concat())
}

fn der_uint(v: &BigUint) -> Vec<u8> {
    let mut b = v.to_bytes_be();
    if b[0] & 0x80 != 0 {
        b.insert(0, 0);
    }
    der(0x02, &b)
}

// --- AES/CBC ---------------------------------------------------------------------------

pub struct Aes {
    key: Vec<u8>,
}

type Enc128 = cbc::Encryptor<aes::Aes128>;
type Dec128 = cbc::Decryptor<aes::Aes128>;
type Enc256 = cbc::Encryptor<aes::Aes256>;
type Dec256 = cbc::Decryptor<aes::Aes256>;

impl Aes {
    pub fn new(key: Vec<u8>) -> Option<Self> {
        matches!(key.len(), 16 | 32).then_some(Self { key })
    }

    /// IV, then the ciphertext.
    pub fn encrypt(&self, plain: &[u8]) -> Vec<u8> {
        let iv = random_bytes(16);
        let mut buf = vec![0u8; plain.len() / 16 * 16 + 16];
        buf[..plain.len()].copy_from_slice(plain);
        let n = if self.key.len() == 16 {
            Enc128::new_from_slices(&self.key, &iv).unwrap().encrypt_padded_mut::<Pkcs7>(&mut buf, plain.len())
        } else {
            Enc256::new_from_slices(&self.key, &iv).unwrap().encrypt_padded_mut::<Pkcs7>(&mut buf, plain.len())
        }
        .expect("the buffer has room for the padding")
        .len();
        buf.truncate(n);
        [iv, buf].concat()
    }

    pub fn decrypt(&self, sealed: &[u8]) -> Option<Vec<u8>> {
        if sealed.len() < 32 || sealed.len() % 16 != 0 {
            return None;
        }
        let (iv, body) = sealed.split_at(16);
        let mut buf = body.to_vec();
        let n = if self.key.len() == 16 {
            Dec128::new_from_slices(&self.key, iv).ok()?.decrypt_padded_mut::<Pkcs7>(&mut buf).ok()?.len()
        } else {
            Dec256::new_from_slices(&self.key, iv).ok()?.decrypt_padded_mut::<Pkcs7>(&mut buf).ok()?.len()
        };
        buf.truncate(n);
        Some(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsa_seals_and_opens_an_aes_key() {
        let key = RsaKey::generate(1024);
        assert_eq!(key.n.bits(), 1024);
        let aes_key = random_bytes(16);
        let sealed = key.encrypt(&aes_key);
        assert_eq!(key.decrypt(&sealed).unwrap(), aes_key);
        assert!(key.decrypt(&sealed[1..]).is_none());
    }

    /// Java reads our public key and seals an AES key with it, as the server
    /// does; we open it. Needs a JDK: `JAVA_INTEROP=1 cargo test -- --ignored`.
    #[test]
    #[ignore = "needs jshell"]
    fn java_seals_to_our_public_key() {
        let key = RsaKey::generate(2048);
        let hex: String = key.public_der().iter().map(|b| format!("{b:02x}")).collect();
        let script = format!(
            "var der = java.util.HexFormat.of().parseHex(\"{hex}\");\n\
             var pk = java.security.KeyFactory.getInstance(\"RSA\").generatePublic(new java.security.spec.X509EncodedKeySpec(der));\n\
             var c = javax.crypto.Cipher.getInstance(\"RSA\"); c.init(javax.crypto.Cipher.ENCRYPT_MODE, pk);\n\
             System.out.println(\"SEALED \" + java.util.HexFormat.of().formatHex(c.doFinal(\"0123456789abcdef\".getBytes())));\n/exit\n"
        );
        let out = std::process::Command::new("jshell")
            .arg("-q")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut c| {
                use std::io::Write;
                c.stdin.take().unwrap().write_all(script.as_bytes())?;
                c.wait_with_output()
            })
            .expect("jshell");
        let text = String::from_utf8_lossy(&out.stdout);
        let sealed_hex = text.split("SEALED ").nth(1).expect("java output").split_whitespace().next().unwrap();
        let sealed: Vec<u8> = (0..sealed_hex.len()).step_by(2).map(|i| u8::from_str_radix(&sealed_hex[i..i + 2], 16).unwrap()).collect();
        assert_eq!(key.decrypt(&sealed).unwrap(), b"0123456789abcdef");
    }

    #[test]
    fn aes_round_trips_and_rejects_tampering() {
        let aes = Aes::new(random_bytes(16)).unwrap();
        let sealed = aes.encrypt(b"opus frame");
        assert_eq!(aes.decrypt(&sealed).unwrap(), b"opus frame");
        let mut bad = sealed.clone();
        bad.truncate(20);
        assert!(aes.decrypt(&bad).is_none());
    }
}
