//! A sentence the screen words itself: a catalog key and named parameters.
//! Rust never builds human text; the UI (`ui/i18n.js`) looks the key up in the
//! language the person chose. A parameter can itself be a `Msg` (an error that
//! names the operation it blocks).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Msg {
    pub key: String,
    #[serde(default)]
    pub params: Map<String, Value>,
}

impl Msg {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            params: Map::new(),
        }
    }

    /// Adds a text or number parameter (`{name}` in the catalog sentence).
    pub fn with(mut self, name: &str, value: impl Into<Value>) -> Self {
        self.params.insert(name.to_string(), value.into());
        self
    }

    /// Adds a parameter that is itself a sentence.
    pub fn with_msg(self, name: &str, message: &Msg) -> Self {
        let value = serde_json::to_value(message).unwrap_or(Value::Null);
        self.with(name, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_serialises_as_key_and_params() {
        let msg = Msg::new("op.send").with("amount", "5 USDC");
        assert_eq!(
            serde_json::to_string(&msg).unwrap(),
            r#"{"key":"op.send","params":{"amount":"5 USDC"}}"#
        );
    }

    #[test]
    fn a_nested_message_keeps_its_own_key_and_params() {
        let inner = Msg::new("op.send").with("amount", "5 USDC");
        let outer = Msg::new("error.pending_blocks").with_msg("what", &inner);
        let json = serde_json::to_value(&outer).unwrap();
        assert_eq!(json["params"]["what"]["key"], "op.send");
        assert_eq!(json["params"]["what"]["params"]["amount"], "5 USDC");
    }

    /// Every catalog key written anywhere in this crate's sources.
    fn keys_in_sources() -> Vec<String> {
        const NAMESPACES: [&str; 7] = [
            "error.",
            "notice.",
            "op.",
            "payee.",
            "history.",
            "resolution.",
            "field.",
        ];
        fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    for (i, piece) in text.split('"').enumerate() {
                        // Odd pieces sit between a pair of quotes.
                        if i % 2 == 1
                            && NAMESPACES.iter().any(|n| piece.starts_with(n))
                            && !piece.ends_with('.')
                            && piece
                                .chars()
                                .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_')
                        {
                            out.push(piece.to_string());
                        }
                    }
                }
            }
        }
        let mut keys = Vec::new();
        walk(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut keys,
        );
        keys.sort();
        keys.dedup();
        keys
    }

    #[test]
    fn every_key_the_app_sends_is_in_both_catalogs() {
        let ui = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/locales");
        let keys = keys_in_sources();
        assert!(keys.len() > 40, "the scan found only {} keys", keys.len());
        for language in ["en", "ru"] {
            let catalog = std::fs::read_to_string(ui.join(format!("{language}.js"))).unwrap();
            for key in &keys {
                assert!(
                    catalog.contains(&format!("\"{key}\":")),
                    "{language} catalog lacks {key}"
                );
            }
        }
    }

    #[test]
    fn params_are_optional_when_reading_a_message() {
        let msg: Msg = serde_json::from_str(r#"{"key":"error.busy"}"#).unwrap();
        assert_eq!(msg, Msg::new("error.busy"));
    }
}
