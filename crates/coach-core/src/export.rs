//! Open-format exports: CSV for any record kind, Sportscode/Nacsport-style XML for tags.

use crate::Result;
use serde_json::Value;
use std::collections::BTreeSet;

/// CSV with one column per field seen in any record (`id` first).
pub fn to_csv(records: &[Value]) -> Result<String> {
    let mut cols: BTreeSet<String> = BTreeSet::new();
    for r in records {
        if let Some(o) = r.as_object() {
            cols.extend(o.keys().filter(|k| *k != "id").cloned());
        }
    }
    let cols: Vec<String> = std::iter::once("id".to_string()).chain(cols).collect();
    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record(&cols)?;
    for r in records {
        let row: Vec<String> = cols
            .iter()
            .map(|c| match &r[c] {
                Value::Null => String::new(),
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect();
        w.write_record(&row)?;
    }
    let bytes = w.into_inner().map_err(|e| crate::Error::Other(e.to_string()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Parse CSV back into records (all values as strings, numbers and booleans detected).
pub fn from_csv(text: &str) -> Result<Vec<serde_json::Map<String, Value>>> {
    let mut r = csv::Reader::from_reader(text.as_bytes());
    let headers = r.headers()?.clone();
    let mut out = Vec::new();
    for rec in r.records() {
        let rec = rec?;
        let mut m = serde_json::Map::new();
        for (h, v) in headers.iter().zip(rec.iter()) {
            if v.is_empty() {
                continue;
            }
            let val = if let Ok(n) = v.parse::<i64>() {
                Value::from(n)
            } else if let Ok(f) = v.parse::<f64>() {
                Value::from(f)
            } else if v == "true" || v == "false" {
                Value::Bool(v == "true")
            } else if v.starts_with('{') || v.starts_with('[') {
                serde_json::from_str(v).unwrap_or_else(|_| Value::String(v.into()))
            } else {
                Value::String(v.into())
            };
            m.insert(h.to_string(), val);
        }
        out.push(m);
    }
    Ok(out)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn hex_to_rgb16(hex: &str) -> (u32, u32, u32) {
    let h = hex.trim_start_matches('#');
    let p = |i: usize| u32::from_str_radix(h.get(i..i + 2).unwrap_or("80"), 16).unwrap_or(128) * 257;
    (p(0), p(2), p(4))
}

/// Sportscode-compatible XML, readable by Nacsport; `colors` maps code -> "#rrggbb".
pub fn tags_to_xml(tags: &[Value], colors: &[(String, String)], player_name: impl Fn(&str) -> String) -> String {
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<file>\n<ALL_INSTANCES>\n");
    for (i, t) in tags.iter().enumerate() {
        let start = t["start"].as_f64().unwrap_or(0.0);
        let end = t["end"].as_f64().unwrap_or(start);
        s += &format!(
            "<instance><ID>{}</ID><start>{:.2}</start><end>{:.2}</end><code>{}</code>",
            i + 1,
            start,
            end,
            esc(t["code"].as_str().unwrap_or(""))
        );
        if let Some(p) = t["player_id"].as_str().filter(|p| !p.is_empty()) {
            s += &format!("<label><group>Player</group><text>{}</text></label>", esc(&player_name(p)));
        }
        if let Some(d) = t["descriptor"].as_str().filter(|d| !d.is_empty()) {
            s += &format!("<label><group>Outcome</group><text>{}</text></label>", esc(d));
        }
        if let Some(p) = t["period"].as_str().filter(|d| !d.is_empty()) {
            s += &format!("<label><group>Period</group><text>{}</text></label>", esc(p));
        }
        s += "</instance>\n";
    }
    s += "</ALL_INSTANCES>\n<ROWS>\n";
    for (code, color) in colors {
        let (r, g, b) = hex_to_rgb16(color);
        s += &format!("<row><code>{}</code><R>{r}</R><G>{g}</G><B>{b}</B></row>\n", esc(code));
    }
    s += "</ROWS>\n</file>\n";
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn csv_roundtrip() {
        let recs = vec![json!({"id": "1", "name": "Ann, B", "number": 7}), json!({"id": "2", "name": "Cy"})];
        let csv = to_csv(&recs).unwrap();
        let back = from_csv(&csv).unwrap();
        assert_eq!(back[0]["name"], "Ann, B");
        assert_eq!(back[0]["number"], 7);
        assert!(back[1].get("number").is_none());
    }

    #[test]
    fn xml_has_instances() {
        let tags = vec![json!({"code": "Shot & goal", "start": 1.0, "end": 9.5, "player_id": "p"})];
        let x = tags_to_xml(&tags, &[("Shot & goal".into(), "#ff0000".into())], |_| "#7".into());
        assert!(x.contains("<code>Shot &amp; goal</code>"));
        assert!(x.contains("<R>65535</R>"));
        assert!(x.contains("<text>#7</text>"));
    }
}
