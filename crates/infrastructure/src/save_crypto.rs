//! Chiffrement et signature de la sauvegarde, pour décourager les retouches à
//! la main.
//!
//! Format : `FSTL1:` suivi, en base64, de `nonce (8 octets) ‖ texte chiffré ‖
//! HMAC-SHA256 (32 octets)`. Le chiffrement est un flux dérivé de SHA-256
//! (clé, nonce, compteur) ; le HMAC couvre le nonce et le texte chiffré. Une
//! seule lettre modifiée et la sauvegarde est refusée.
//!
//! La clé des versions officielles est injectée à la compilation par la CI
//! (`FONDERIE_SAVE_KEY`, secret du dépôt) ; une compilation locale utilise une
//! clé de développement.

use sha2::{Digest, Sha256};

/// En-tête d'une sauvegarde chiffrée.
pub const MAGIC: &str = "FSTL1:";
const NONCE_LEN: usize = 8;
const MAC_LEN: usize = 32;
const DEV_KEY: &str = "fonderie-stellaire/cle-de-developpement";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Pas une sauvegarde chiffrée (ancienne sauvegarde en clair ?).
    NotEncrypted,
    /// Illisible ou modifiée : la signature ne correspond pas.
    Tampered,
}

fn key() -> [u8; 32] {
    let secret = match option_env!("FONDERIE_SAVE_KEY") {
        Some(k) if !k.trim().is_empty() => k,
        _ => DEV_KEY,
    };
    Sha256::digest(secret.as_bytes()).into()
}

/// Flux pseudo-aléatoire : SHA-256(clé ‖ nonce ‖ compteur), bloc après bloc.
fn keystream_xor(key: &[u8; 32], nonce: &[u8], data: &mut [u8]) {
    for (counter, chunk) in data.chunks_mut(32).enumerate() {
        let mut h = Sha256::new();
        h.update(key);
        h.update(nonce);
        h.update((counter as u64).to_le_bytes());
        let block = h.finalize();
        for (b, k) in chunk.iter_mut().zip(block.iter()) {
            *b ^= k;
        }
    }
}

/// HMAC-SHA256 (RFC 2104).
pub fn hmac(key: &[u8; 32], parts: &[&[u8]]) -> [u8; 32] {
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for (i, k) in key.iter().enumerate() {
        ipad[i] ^= k;
        opad[i] ^= k;
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    for p in parts {
        inner.update(p);
    }
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner.finalize());
    outer.finalize().into()
}

/// Chiffre et signe `plain` ; `nonce` doit changer à chaque sauvegarde.
pub fn encode(plain: &str, nonce: u64) -> String {
    let key = key();
    let out = seal(&key, &key, plain.as_bytes(), nonce);
    format!("{MAGIC}{}", base64_encode(&out))
}

/// Vérifie la signature puis déchiffre.
pub fn decode(text: &str) -> Result<String, CryptoError> {
    let body = text
        .trim()
        .strip_prefix(MAGIC)
        .ok_or(CryptoError::NotEncrypted)?;
    let raw = base64_decode(body).ok_or(CryptoError::Tampered)?;
    let key = key();
    let plain = open(&key, &key, &raw)?;
    String::from_utf8(plain).map_err(|_| CryptoError::Tampered)
}

/// `nonce (8 octets) ‖ texte chiffré ‖ HMAC-SHA256(mac_key, nonce ‖ chiffré)`.
pub fn seal(enc_key: &[u8; 32], mac_key: &[u8; 32], plain: &[u8], nonce: u64) -> Vec<u8> {
    let nonce = nonce.to_le_bytes();
    let mut data = plain.to_vec();
    keystream_xor(enc_key, &nonce, &mut data);
    let mac = hmac(mac_key, &[&nonce, &data]);
    let mut out = Vec::with_capacity(NONCE_LEN + data.len() + MAC_LEN);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&data);
    out.extend_from_slice(&mac);
    out
}

/// Inverse de [`seal`] : refuse tout contenu dont la signature ne correspond pas.
pub fn open(enc_key: &[u8; 32], mac_key: &[u8; 32], sealed: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if sealed.len() < NONCE_LEN + MAC_LEN {
        return Err(CryptoError::Tampered);
    }
    let (nonce, rest) = sealed.split_at(NONCE_LEN);
    let (data, mac) = rest.split_at(rest.len() - MAC_LEN);
    let expected = hmac(mac_key, &[nonce, data]);
    // Comparaison en temps constant.
    let diff = expected
        .iter()
        .zip(mac.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b));
    if diff != 0 {
        return Err(CryptoError::Tampered);
    }
    let mut plain = data.to_vec();
    keystream_xor(enc_key, nonce, &mut plain);
    Ok(plain)
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let clean: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !clean.len().is_multiple_of(4) {
        return None;
    }
    let value = |c: u8| -> Option<u32> { ALPHABET.iter().position(|&a| a == c).map(|p| p as u32) };
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks(4) {
        let pad = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 {
            return None;
        }
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            let v = if i >= 4 - pad { 0 } else { value(c)? };
            n = (n << 6) | v;
        }
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..3 - pad]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_unreadable_without_the_key() {
        let plain = r#"{"stardust": 123.0, "clicks": 7}"#;
        let enc = encode(plain, 42);
        assert!(enc.starts_with(MAGIC));
        assert!(!enc.contains("stardust"), "illisible en clair");
        assert_eq!(decode(&enc).unwrap(), plain);
        // Même contenu, autre nonce : autre texte.
        assert_ne!(encode(plain, 43), enc);
    }

    #[test]
    fn any_edit_is_detected() {
        let enc = encode(r#"{"fragments": 10.0}"#, 1);
        let mut bytes = enc.into_bytes();
        let i = bytes.len() / 2;
        bytes[i] = if bytes[i] == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert_eq!(decode(&tampered), Err(CryptoError::Tampered));
        assert_eq!(decode("{\"clair\": 1}"), Err(CryptoError::NotEncrypted));
        assert_eq!(decode("FSTL1:@@@@"), Err(CryptoError::Tampered));
    }

    #[test]
    fn sealed_bytes_need_both_keys() {
        let (k1, k2) = ([1u8; 32], [2u8; 32]);
        let sealed = seal(&k1, &k2, b"partie", 9);
        assert_eq!(open(&k1, &k2, &sealed).unwrap(), b"partie");
        assert_ne!(open(&k2, &k2, &sealed), Ok(b"partie".to_vec()));
        assert_eq!(open(&k1, &k1, &sealed), Err(CryptoError::Tampered));
        assert_eq!(open(&k1, &k2, &sealed[..10]), Err(CryptoError::Tampered));
    }

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"M"), "TQ==");
        for s in ["", "a", "ab", "abc", "abcd", "fonderie ✦ stellaire"] {
            assert_eq!(
                base64_decode(&base64_encode(s.as_bytes())).unwrap(),
                s.as_bytes()
            );
        }
    }
}
