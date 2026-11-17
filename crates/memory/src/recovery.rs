//! Versioned recovery material shared by copyable text and QR encoding.
//!
//! This is not connected to a UI or key store. The caller must protect the returned secret.
use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use zeroize::Zeroizing;

const PREFIX: &str = "footnote-recovery-v1";
const DOMAIN: &[u8] = b"footnote-recovery-v1 checksum\0";

/// Generate a key using OS entropy. No key is written to persistent storage.
pub fn generate() -> Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0_u8; 32]);
    getrandom::fill(&mut *key).map_err(|e| anyhow::anyhow!("OS entropy unavailable: {e}"))?;
    Ok(key)
}

fn checksum(key: &[u8; 32]) -> [u8; 8] {
    let mut hash = Sha256::new();
    hash.update(DOMAIN);
    hash.update(key);
    let digest = hash.finalize();
    digest[..8].try_into().expect("SHA-256 has 32 bytes")
}

/// The returned string is secret; its checksum detects errors, not tampering.
pub fn encode(key: &[u8; 32]) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(PREFIX.len() + 82));
    out.push_str(PREFIX);
    out.push(':');
    for byte in key {
        write!(&mut out, "{byte:02x}").expect("writing to a String cannot fail");
    }
    out.push(':');
    for byte in checksum(key) {
        write!(&mut out, "{byte:02x}").expect("writing to a String cannot fail");
    }
    out
}

/// Return a square matrix of 0 (light) and 1 (dark) modules. The matrix is the secret.
/// The eventual renderer must add a four-module quiet zone and protect the displayed image.
pub fn qr(key: &[u8; 32]) -> Result<(usize, Zeroizing<Vec<u8>>)> {
    let payload = encode(key);
    let code = qrcode::QrCode::new(payload.as_bytes())?;
    let width = code.width();
    let mut modules = Zeroizing::new(Vec::with_capacity(width * width));
    for y in 0..width {
        for x in 0..width {
            modules.push(u8::from(code[(x, y)] == qrcode::Color::Dark));
        }
    }
    Ok((width, modules))
}

fn hex_pair(a: u8, b: u8) -> Option<u8> {
    fn digit(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        }
    }
    Some(digit(a)? * 16 + digit(b)?)
}

/// Decode only the canonical v1 format. Error messages never contain input material.
pub fn decode(input: &str) -> Result<Zeroizing<[u8; 32]>> {
    let mut parts = input.split(':');
    let (Some(prefix), Some(hex), Some(check), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        bail!("invalid recovery key format");
    };
    if prefix != PREFIX || hex.len() != 64 || check.len() != 16 {
        bail!("invalid recovery key format or version");
    }
    let mut key = Zeroizing::new([0_u8; 32]);
    for (dest, pair) in key.iter_mut().zip(hex.as_bytes().as_chunks::<2>().0) {
        *dest = hex_pair(pair[0], pair[1])
            .ok_or_else(|| anyhow::anyhow!("invalid recovery key encoding"))?;
    }
    let mut given = [0_u8; 8];
    for (dest, pair) in given.iter_mut().zip(check.as_bytes().as_chunks::<2>().0) {
        *dest = hex_pair(pair[0], pair[1])
            .ok_or_else(|| anyhow::anyhow!("invalid recovery key encoding"))?;
    }
    if given != checksum(&key) {
        bail!("recovery key checksum does not match");
    }
    Ok(key)
}
