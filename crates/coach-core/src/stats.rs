//! Stats from tags: counts per code, player and period, and descriptor outcomes.

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub fn from_tags(tags: &[Value], players: &[Value]) -> Value {
    let name_of = |id: &str| -> String {
        players
            .iter()
            .find(|p| p["id"] == id)
            .map(|p| match (p["number"].as_i64(), p["name"].as_str()) {
                (Some(n), Some(name)) => format!("#{n} {name}"),
                (None, Some(name)) => name.to_string(),
                _ => id.to_string(),
            })
            .unwrap_or_else(|| id.to_string())
    };

    let mut by_code: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_period: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut by_player: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut outcomes: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();

    for t in tags {
        let code = t["code"].as_str().unwrap_or("?").to_string();
        *by_code.entry(code.clone()).or_default() += 1;
        let period = t["period"].as_str().unwrap_or("—").to_string();
        *by_period.entry(period).or_default().entry(code.clone()).or_default() += 1;
        if let Some(pid) = t["player_id"].as_str().filter(|s| !s.is_empty()) {
            *by_player.entry(name_of(pid)).or_default().entry(code.clone()).or_default() += 1;
        }
        if let Some(d) = t["descriptor"].as_str().filter(|s| !s.is_empty()) {
            *outcomes.entry(code).or_default().entry(d.to_string()).or_default() += 1;
        }
    }

    let nested = |m: BTreeMap<String, BTreeMap<String, u64>>| -> Value {
        Value::Object(
            m.into_iter()
                .map(|(k, v)| (k, json!(v)))
                .collect::<Map<String, Value>>(),
        )
    };

    json!({
        "total": tags.len(),
        "by_code": by_code,
        "by_period": nested(by_period),
        "by_player": nested(by_player),
        "outcomes": nested(outcomes),
    })
}

/// Minutes played per player from start / sub_on / sub_off / end events.
pub fn minutes_played(events: &[Value]) -> BTreeMap<String, f64> {
    let mut sorted: Vec<&Value> = events.iter().collect();
    sorted.sort_by(|a, b| {
        a["minute"].as_f64().unwrap_or(0.0).total_cmp(&b["minute"].as_f64().unwrap_or(0.0))
    });
    let mut on: BTreeMap<String, f64> = BTreeMap::new();
    let mut total: BTreeMap<String, f64> = BTreeMap::new();
    for e in sorted {
        let m = e["minute"].as_f64().unwrap_or(0.0);
        let pid = e["player_id"].as_str().unwrap_or("").to_string();
        match e["kind"].as_str().unwrap_or("") {
            "start" | "sub_on" if !pid.is_empty() => {
                on.insert(pid, m);
            }
            "sub_off" | "red_card" if !pid.is_empty() => {
                if let Some(s) = on.remove(&pid) {
                    *total.entry(pid).or_default() += m - s;
                }
            }
            "end" => {
                for (p, s) in std::mem::take(&mut on) {
                    *total.entry(p).or_default() += m - s;
                }
            }
            _ => {}
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_minutes() {
        let tags = vec![
            json!({"code": "Shot", "player_id": "p1", "period": "1st half", "descriptor": "Success"}),
            json!({"code": "Shot", "player_id": "p1", "period": "2nd half", "descriptor": "Fail"}),
            json!({"code": "Turnover", "period": "2nd half"}),
        ];
        let players = vec![json!({"id": "p1", "name": "Ann", "number": 7})];
        let s = from_tags(&tags, &players);
        assert_eq!(s["by_code"]["Shot"], 2);
        assert_eq!(s["by_player"]["#7 Ann"]["Shot"], 2);
        assert_eq!(s["outcomes"]["Shot"]["Fail"], 1);

        let ev = vec![
            json!({"kind": "start", "player_id": "a", "minute": 0}),
            json!({"kind": "start", "player_id": "b", "minute": 0}),
            json!({"kind": "sub_off", "player_id": "b", "minute": 60}),
            json!({"kind": "sub_on", "player_id": "c", "minute": 60}),
            json!({"kind": "end", "minute": 90}),
        ];
        let m = minutes_played(&ev);
        assert_eq!(m["a"], 90.0);
        assert_eq!(m["b"], 60.0);
        assert_eq!(m["c"], 30.0);
    }
}
