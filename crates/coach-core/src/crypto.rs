//! Shared-key encryption for sync traffic (ChaCha20-Poly1305) and length-prefixed framing.

use crate::{Error, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use std::io::{Read, Write};

pub type SyncKey = [u8; 32];
const NONCE_LEN: usize = 12;
/// Largest frame we accept (protects against garbage / hostile peers).
pub const MAX_FRAME: usize = 8 * 1024 * 1024;

pub fn generate_key() -> SyncKey {
    let mut k = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut k);
    k
}

pub fn key_to_string(k: &SyncKey) -> String {
    B64.encode(k)
}

pub fn key_from_string(s: &str) -> Result<SyncKey> {
    let v = B64.decode(s.trim()).map_err(|_| Error::Crypto)?;
    v.try_into().map_err(|_| Error::Crypto)
}

pub fn seal(key: &SyncKey, plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| Error::Crypto)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn open(key: &SyncKey, data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < NONCE_LEN {
        return Err(Error::Crypto);
    }
    let (nonce, ct) = data.split_at(NONCE_LEN);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(nonce), ct)
        .map_err(|_| Error::Crypto)
}

pub fn write_frame<W: Write>(w: &mut W, payload: &[u8]) -> Result<()> {
    if payload.len() > MAX_FRAME {
        return Err(Error::Protocol("frame too large".into()));
    }
    w.write_all(&(payload.len() as u32).to_be_bytes())?;
    w.write_all(payload)?;
    w.flush()?;
    Ok(())
}

pub fn read_frame<R: Read>(r: &mut R) -> Result<Vec<u8>> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(Error::Protocol("frame too large".into()));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_roundtrip_and_wrong_key_fails() {
        let k = generate_key();
        let sealed = seal(&k, b"hello coach").unwrap();
        assert_eq!(open(&k, &sealed).unwrap(), b"hello coach");
        assert!(open(&generate_key(), &sealed).is_err());
        let s = key_to_string(&k);
        assert_eq!(key_from_string(&s).unwrap(), k);
    }
}
