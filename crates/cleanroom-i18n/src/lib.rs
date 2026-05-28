//! cleanroom-i18n — Internationalization support for Cleanroom Agent.
//!
//! Provides:
//! - `Lang` enum for available languages (En, ZhCn)
//! - `Translator` that loads locale JSON at compile time
//! - `tr!()` macro for string lookup with positional format arguments
//! - Auto-detection from `LANG` / `CLEANROOM_LANG` environment variables
//!
//! ## Usage
//! ```ignore
//! use cleanroom_i18n::{tr, Translator, Lang};
//!
//! let t = Translator::new(Lang::ZhCn);
//! println!("{}", tr!(t, "cli.produce_about"));
//! println!("{}", tr!(t, "cli.produce_complete", "my-project"));
//! ```

use std::collections::HashMap;
use std::sync::OnceLock;

/// Available languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// English
    En,
    /// Chinese (Simplified)
    ZhCn,
}

impl Lang {
    /// Detect language from environment variables.
    /// Priority: `CLEANROOM_LANG` > `LANG`
    pub fn from_env() -> Self {
        if let Ok(val) = std::env::var("CLEANROOM_LANG") {
            return Self::from_str(&val);
        }
        if let Ok(val) = std::env::var("LANG") {
            return Self::from_str(&val);
        }
        Self::En
    }

    /// Parse from a string code.
    pub fn from_str(s: &str) -> Self {
        let s = s.to_lowercase();
        if s.contains("zh") || s == "chinese" {
            Self::ZhCn
        } else {
            Self::En
        }
    }

    /// Get the locale file name.
    pub fn locale_file(self) -> &'static str {
        match self {
            Self::En => include_str!("../locales/en.json"),
            Self::ZhCn => include_str!("../locales/zh-CN.json"),
        }
    }

    /// Get all available languages.
    pub fn all() -> &'static [Lang] {
        &[Lang::En, Lang::ZhCn]
    }

    /// Display name in the language itself.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::ZhCn => "中文",
        }
    }
}

/// Thread-safe global translator.
static GLOBAL_TRANSLATOR: OnceLock<Translator> = OnceLock::new();

/// Initialize the global translator (called once at program start).
pub fn init(lang: Lang) {
    let _ = GLOBAL_TRANSLATOR.set(Translator::new(lang));
}

/// Get the global translator.
pub fn global() -> &'static Translator {
    GLOBAL_TRANSLATOR.get_or_init(|| {
        let lang = Lang::from_env();
        Translator::new(lang)
    })
}

/// Flattened locale: all keys are dot-separated paths.
type LocaleMap = HashMap<String, String>;

/// Translator — resolves translation keys to localized strings.
#[derive(Debug, Clone)]
pub struct Translator {
    lang: Lang,
    map: LocaleMap,
}

impl Translator {
    /// Create a new translator for the given language.
    pub fn new(lang: Lang) -> Self {
        let raw = lang.locale_file();
        let parsed: serde_json::Value = serde_json::from_str(raw)
            .expect("Invalid locale JSON file");
        let map = flatten_value(&parsed, String::new());
        Self { lang, map }
    }

    /// Get the current language.
    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// Translate a dot-separated key.
    /// Returns the key itself if not found (graceful fallback).
    pub fn translate(&self, key: &str) -> String {
        self.map
            .get(key)
            .cloned()
            .unwrap_or_else(|| {
                // Fallback: try English
                let en = Translator::new(Lang::En);
                en.map.get(key).cloned().unwrap_or_else(|| key.to_string())
            })
    }

    /// Translate with positional format arguments ({0}, {1}, ...).
    pub fn translate_with_args(&self, key: &str, args: &[&dyn std::fmt::Display]) -> String {
        let mut s = self.translate(key);
        for (i, arg) in args.iter().enumerate() {
            s = s.replace(&format!("{{{}}}", i), &arg.to_string());
        }
        s
    }
}

/// Flatten a nested JSON object into a dot-separated key map.
fn flatten_value(value: &serde_json::Value, prefix: String) -> LocaleMap {
    let mut map = LocaleMap::new();
    match value {
        serde_json::Value::Object(obj) => {
            for (key, val) in obj {
                let new_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                map.extend(flatten_value(val, new_key));
            }
        }
        serde_json::Value::String(s) => {
            map.insert(prefix, s.clone());
        }
        _ => {
            map.insert(prefix, value.to_string());
        }
    }
    map
}

/// Translate a key using a translator instance.
#[macro_export]
macro_rules! tr {
    ($t:expr, $key:literal) => { $t.translate($key) };
    ($t:expr, $key:literal, $($arg:expr),+) => {{
        let args: &[&dyn std::fmt::Display] = &[$(&$arg),+];
        $t.translate_with_args($key, args)
    }};
}

/// Translate using the global translator.
#[macro_export]
macro_rules! tr_global {
    ($key:literal) => { $crate::global().translate($key) };
    ($key:literal, $($arg:expr),+) => {{
        let args: &[&dyn std::fmt::Display] = &[$(&$arg),+];
        $crate::global().translate_with_args($key, args)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_translator() {
        let t = Translator::new(Lang::En);
        assert_eq!(t.translate("cli.app_about"), "Cleanroom Agent — S.DEF intelligent agent system");
    }

    #[test]
    fn test_chinese_translator() {
        let t = Translator::new(Lang::ZhCn);
        assert_eq!(t.translate("cli.produce_about"), "生产模式：分析代码仓库 → 输出 S.DEF");
    }

    #[test]
    fn test_translate_with_args() {
        let t = Translator::new(Lang::En);
        let s = t.translate_with_args("cli.produce_complete", &[&"my-project"]);
        assert_eq!(s, "Production completed for 'my-project'");
    }

    #[test]
    fn test_key_fallback_to_english() {
        // A key that only exists in English should fall back
        let t = Translator::new(Lang::ZhCn);
        // All keys exist in both locales, so this tests the key-itself fallback for missing keys
        let s = t.translate("nonexistent.key");
        assert_eq!(s, "nonexistent.key");
    }

    #[test]
    fn test_lang_from_env() {
        // No env set — defaults to English
        let lang = Lang::from_env();
        // Just ensure it doesn't panic
        let _ = lang;
    }

    #[test]
    fn test_global_translator() {
        init(Lang::En);
        let t = global();
        assert_eq!(t.lang(), Lang::En);
    }

    #[test]
    fn test_mcp_descriptions_exist() {
        let t_en = Translator::new(Lang::En);
        let t_zh = Translator::new(Lang::ZhCn);

        let tools = [
            "mcp.create_task", "mcp.get_data_model", "mcp.resolve_name",
            "mcp.export_sdef", "mcp.check_consistency", "mcp.begin_transaction",
        ];

        for key in &tools {
            let en = t_en.translate(key);
            let zh = t_zh.translate(key);
            assert_ne!(en, *key, "English key '{}' not found", key);
            assert_ne!(zh, *key, "Chinese key '{}' not found", key);
        }
    }
}
