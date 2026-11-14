
use serde_json::Value;

const BUILTIN: &[(&str, &str)] = &[
    ("football", include_str!("../../../templates/football.json")),
    ("basketball", include_str!("../../../templates/basketball.json")),
    ("rugby", include_str!("../../../templates/rugby.json")),
    ("hockey", include_str!("../../../templates/hockey.json")),
    ("volleyball", include_str!("../../../templates/volleyball.json")),
    ("netball", include_str!("../../../templates/netball.json")),
];

pub fn builtin() -> Vec<Value> {
    BUILTIN
        .iter()
        .filter_map(|(_, s)| serde_json::from_str(s).ok())
        .collect()
}

pub fn get(id: &str) -> Option<Value> {
    BUILTIN
        .iter()
        .find(|(k, _)| *k == id)
        .and_then(|(_, s)| serde_json::from_str(s).ok())
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_templates_parse_and_have_panels() {
        let all = super::builtin();
        assert_eq!(all.len(), 6);
        for t in all {
            assert!(t["panel"]["buttons"].as_array().unwrap().len() >= 5, "{}", t["id"]);
        }
    }
}
