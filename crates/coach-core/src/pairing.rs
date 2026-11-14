//! What the desktop's pairing QR code carries.

use crate::{Error, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 47820;
const PREFIX: &str = "coachpb://pair?d=";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PairingInfo {
    pub v: u32,
    /// Every LAN address the desktop is reachable on (Wi-Fi, hotspot, Ethernet).
    pub hosts: Vec<String>,
    pub port: u16,
    /// One-time key; becomes the shared key stored for this phone.
    pub key: String,
    pub device_id: String,
    pub name: String,
    pub team_id: String,
    pub team_name: String,
}

impl PairingInfo {
    pub fn to_uri(&self) -> String {
        let json = serde_json::to_vec(self).expect("serializable");
        format!("{PREFIX}{}", B64.encode(json))
    }

    pub fn from_uri(s: &str) -> Result<Self> {
        let d = s
            .trim()
            .strip_prefix(PREFIX)
            .ok_or_else(|| Error::Other("not a Coach's Playbook pairing code".into()))?;
        let bytes = B64
            .decode(d)
            .map_err(|_| Error::Other("damaged pairing code".into()))?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_roundtrip() {
        let p = PairingInfo {
            v: 1,
            hosts: vec!["192.168.1.10".into()],
            port: DEFAULT_PORT,
            key: "k".into(),
            device_id: "d".into(),
            name: "Coach PC".into(),
            team_id: "t".into(),
            team_name: "U14".into(),
        };
        assert_eq!(PairingInfo::from_uri(&p.to_uri()).unwrap(), p);
    }
}
