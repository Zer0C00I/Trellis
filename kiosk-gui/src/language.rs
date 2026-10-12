//! Which language the kiosk speaks, decided once at start-up.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Ru,
    En,
}

impl Language {
    /// The folder name under `translations/` and what
    /// `slint::select_bundled_translation` expects.
    pub fn code(self) -> &'static str {
        match self {
            Language::Ru => "ru",
            Language::En => "en",
        }
    }

    fn parse(text: &str) -> Option<Language> {
        match text.trim().to_ascii_lowercase().as_str() {
            "ru" => Some(Language::Ru),
            "en" => Some(Language::En),
            _ => None,
        }
    }
}

/// The configured value if it names a supported language, otherwise the
/// prefix of `LANG` (`ru_RU.UTF-8` is `ru`), otherwise Russian, which is
/// what every already-installed kiosk speaks.
pub fn resolve_language(configured: Option<&str>, lang_env: Option<&str>) -> Language {
    configured
        .and_then(Language::parse)
        .or_else(|| lang_env.and_then(language_from_env))
        .unwrap_or(Language::Ru)
}

fn language_from_env(lang: &str) -> Option<Language> {
    let prefix: String = lang
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    Language::parse(&prefix)
}

/// The string `language` of a TOML file (`kiosk.toml`, `provisioning.toml`).
/// `None` when the file is missing, is not TOML, or has no such string.
pub fn language_setting(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let table: toml::Table = text.parse().ok()?;
    table.get("language")?.as_str().map(str::to_string)
}

/// The language to start with. `kiosk.toml` wins; on a first boot, which has
/// no `kiosk.toml` yet, `provisioning.toml` next to it is used.
pub fn startup_language(config_path: &Path, lang_env: Option<&str>) -> Language {
    let provisioning = config_path
        .parent()
        .map(|dir| dir.join("provisioning.toml"));
    let configured =
        language_setting(config_path).or_else(|| provisioning.and_then(|p| language_setting(&p)));
    if let Some(value) = &configured {
        if Language::parse(value).is_none() {
            eprintln!(
                "[kiosk-gui] unknown language {value:?} in the configuration; using the system language"
            );
        }
    }
    resolve_language(configured.as_deref(), lang_env)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kiosk-language-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_configured_language_wins_over_the_environment() {
        assert_eq!(
            resolve_language(Some("en"), Some("ru_RU.UTF-8")),
            Language::En
        );
        assert_eq!(
            resolve_language(Some("ru"), Some("en_US.UTF-8")),
            Language::Ru
        );
    }

    #[test]
    fn the_configured_value_is_case_and_space_insensitive() {
        assert_eq!(resolve_language(Some(" EN "), None), Language::En);
        assert_eq!(resolve_language(Some("Ru"), None), Language::Ru);
    }

    #[test]
    fn an_unsupported_configured_value_falls_back_to_the_environment() {
        assert_eq!(
            resolve_language(Some("de"), Some("en_GB.UTF-8")),
            Language::En
        );
        assert_eq!(resolve_language(Some(""), Some("en_US")), Language::En);
    }

    #[test]
    fn the_environment_prefix_selects_the_language() {
        assert_eq!(resolve_language(None, Some("ru_RU.UTF-8")), Language::Ru);
        assert_eq!(resolve_language(None, Some("en_US.UTF-8")), Language::En);
        assert_eq!(resolve_language(None, Some("en")), Language::En);
    }

    #[test]
    fn everything_else_is_russian() {
        assert_eq!(resolve_language(None, None), Language::Ru);
        assert_eq!(resolve_language(None, Some("C")), Language::Ru);
        assert_eq!(resolve_language(None, Some("POSIX")), Language::Ru);
        assert_eq!(resolve_language(None, Some("fr_FR.UTF-8")), Language::Ru);
        assert_eq!(resolve_language(None, Some("")), Language::Ru);
    }

    #[test]
    fn the_code_is_the_translation_folder_name() {
        assert_eq!(Language::Ru.code(), "ru");
        assert_eq!(Language::En.code(), "en");
    }

    #[test]
    fn language_setting_reads_a_string_and_ignores_everything_else() {
        let dir = temp_dir("setting");
        let path = dir.join("kiosk.toml");
        std::fs::write(&path, "rpc_url = \"x\"\nlanguage = \"en\"\n").unwrap();
        assert_eq!(language_setting(&path).as_deref(), Some("en"));
        std::fs::write(&path, "rpc_url = \"x\"\n").unwrap();
        assert_eq!(language_setting(&path), None);
        std::fs::write(&path, "language = 5\n").unwrap();
        assert_eq!(language_setting(&path), None);
        std::fs::write(&path, "this is not toml [[[").unwrap();
        assert_eq!(language_setting(&path), None);
        assert_eq!(language_setting(&dir.join("missing.toml")), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn kiosk_toml_beats_provisioning_toml_and_provisioning_beats_the_environment() {
        let dir = temp_dir("startup");
        let kiosk = dir.join("kiosk.toml");
        std::fs::write(dir.join("provisioning.toml"), "language = \"en\"\n").unwrap();

        // First boot: no kiosk.toml, so provisioning.toml decides.
        assert_eq!(startup_language(&kiosk, Some("ru_RU.UTF-8")), Language::En);

        std::fs::write(&kiosk, "language = \"ru\"\n").unwrap();
        assert_eq!(startup_language(&kiosk, Some("en_US.UTF-8")), Language::Ru);

        std::fs::remove_file(&kiosk).unwrap();
        std::fs::remove_file(dir.join("provisioning.toml")).unwrap();
        assert_eq!(startup_language(&kiosk, Some("en_US.UTF-8")), Language::En);
        assert_eq!(startup_language(&kiosk, None), Language::Ru);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
