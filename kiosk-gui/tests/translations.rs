//! Every phrase in the `.slint` files must have a translation, and the
//! translation file must not carry phrases nothing uses.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn unescape(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// The first string literal of every `@tr(...)` in the file.
fn slint_phrases(source: &str) -> Vec<String> {
    let mut phrases = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("@tr(") {
        rest = &rest[at + 4..];
        let rest_trimmed = rest.trim_start();
        let Some(body) = rest_trimmed.strip_prefix('"') else {
            continue;
        };
        let mut end = None;
        let mut escaped = false;
        for (i, c) in body.char_indices() {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                end = Some(i);
                break;
            }
        }
        let end = end.expect("unterminated @tr string");
        phrases.push(unescape(&body[..end]));
    }
    phrases
}

/// `msgid` -> `msgstr` of a `.po` file, without the header entry.
fn po_entries(source: &str) -> BTreeMap<String, String> {
    let mut entries = BTreeMap::new();
    let (mut id, mut value, mut current): (Option<String>, Option<String>, Option<bool>) =
        (None, None, None);
    let mut flush = |id: &mut Option<String>, value: &mut Option<String>| {
        if let (Some(i), Some(v)) = (id.take(), value.take()) {
            if !i.is_empty() {
                entries.insert(i, v);
            }
        }
    };
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("msgid ") {
            flush(&mut id, &mut value);
            id = Some(unescape(rest.trim().trim_matches('"')));
            current = Some(true);
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            value = Some(unescape(rest.trim().trim_matches('"')));
            current = Some(false);
        } else if line.starts_with('"') {
            let piece = unescape(line.trim_matches('"'));
            match current {
                Some(true) => id.as_mut().unwrap().push_str(&piece),
                Some(false) => value.as_mut().unwrap().push_str(&piece),
                None => {}
            }
        }
    }
    flush(&mut id, &mut value);
    entries
}

fn used_phrases() -> BTreeSet<String> {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut phrases = BTreeSet::new();
    for entry in std::fs::read_dir(ui).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "slint") {
            let source = std::fs::read_to_string(&path).unwrap();
            phrases.extend(slint_phrases(&source));
        }
    }
    phrases
}

fn russian() -> BTreeMap<String, String> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("translations/ru/LC_MESSAGES/kiosk-gui.po");
    po_entries(&std::fs::read_to_string(path).unwrap())
}

#[test]
fn every_phrase_in_the_ui_is_translated_into_russian() {
    let translations = russian();
    let missing: Vec<_> = used_phrases()
        .into_iter()
        .filter(|p| translations.get(p).is_none_or(|t| t.trim().is_empty()))
        .collect();
    assert!(
        missing.is_empty(),
        "no Russian translation for: {missing:#?}"
    );
}

#[test]
fn the_russian_file_has_no_phrases_the_ui_no_longer_uses() {
    let used = used_phrases();
    let stale: Vec<_> = russian()
        .into_keys()
        .filter(|p| !used.contains(p))
        .collect();
    assert!(stale.is_empty(), "unused translations: {stale:#?}");
}

#[test]
fn placeholders_match_between_a_phrase_and_its_translation() {
    let count = |s: &str| s.matches("{}").count();
    for (phrase, translation) in russian() {
        assert_eq!(
            count(&phrase),
            count(&translation),
            "`{phrase}` and `{translation}` must have the same number of {{}} placeholders"
        );
    }
}

#[test]
fn the_phrase_extractor_handles_escapes_and_arguments() {
    let source = r#"text: @tr("Send {} \"now\"", a, b); x: @tr( "Plain" );"#;
    assert_eq!(slint_phrases(source), vec!["Send {} \"now\"", "Plain"]);
}

#[test]
fn the_po_reader_joins_continuation_lines_and_skips_the_header() {
    let po = "msgid \"\"\nmsgstr \"\"\n\"Language: ru\\n\"\n\nmsgid \"Hello \"\n\"world\"\nmsgstr \"Привет\"\n\" мир\"\n";
    let entries = po_entries(po);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries["Hello world"], "Привет мир");
}
