use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    ZhCN,
    #[serde(rename = "en-US")]
    EnUS,
}

impl Language {
    /// Get language code (e.g., "zh-CN", "en-US")
    #[cfg(test)]
    pub fn code(&self) -> &'static str {
        match self {
            Language::ZhCN => "zh-CN",
            Language::EnUS => "en-US",
        }
    }

    /// Get language display name
    pub fn name(&self) -> &'static str {
        match self {
            Language::ZhCN => "中文",
            Language::EnUS => "English",
        }
    }

    /// Detect system language
    pub fn detect_system() -> Self {
        match sys_locale::get_locale().as_deref() {
            Some(locale) if locale.starts_with("zh") => Language::ZhCN,
            _ => Language::EnUS, // Default to English
        }
    }
}

/// Internationalization manager
pub struct I18n {
    translations: HashMap<String, serde_json::Value>,
    language: Language,
}

impl I18n {
    /// Create a new I18n instance with the specified language
    pub fn new(language: Language) -> Result<Self> {
        let json_content = match language {
            Language::ZhCN => include_str!("../locales/zh-CN.json"),
            Language::EnUS => include_str!("../locales/en-US.json"),
        };

        let translations: HashMap<String, serde_json::Value> = serde_json::from_str(json_content)?;

        Ok(Self { translations, language })
    }

    /// Current UI language (drives localized metadata name display)
    pub fn language(&self) -> Language {
        self.language
    }

    /// Translate a key to the current language
    /// Supports nested keys like "menu.file.open"
    pub fn t(&self, key: &str) -> String {
        let parts: Vec<&str> = key.split('.').collect();
        let mut value = &serde_json::Value::Object(
            self.translations.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        );

        for part in &parts {
            value = match value.get(part) {
                Some(v) => v,
                None => return format!("[Missing: {}]", key),
            };
        }

        value.as_str().unwrap_or(&format!("[Invalid: {}]", key)).to_string()
    }

    /// Set language (reloads translations)
    pub fn set_language(&mut self, language: Language) -> Result<()> {
        *self = Self::new(language)?;
        Ok(())
    }
}

/// Metadata display name for the current UI language: zh-CN shows the
/// Chinese name when present (falls back to the English canonical name,
/// which also marks the entry as not yet localized); en-US always shows
/// the English canonical name.
pub fn localized_name(language: Language, names: &vlabel_core::LocalizedNames) -> &str {
    match (language, &names.zh) {
        (Language::ZhCN, Some(zh)) => zh,
        _ => &names.en,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_code() {
        assert_eq!(Language::ZhCN.code(), "zh-CN");
        assert_eq!(Language::EnUS.code(), "en-US");
    }

    #[test]
    fn test_language_name() {
        assert_eq!(Language::ZhCN.name(), "中文");
        assert_eq!(Language::EnUS.name(), "English");
    }

    #[test]
    fn localized_name_zh_prefers_zh() {
        let names = vlabel_core::LocalizedNames {
            en: "trash".to_string(),
            zh: Some("垃圾桶".to_string()),
        };
        assert_eq!(localized_name(Language::ZhCN, &names), "垃圾桶");
    }

    #[test]
    fn localized_name_zh_falls_back_to_en() {
        let names = vlabel_core::LocalizedNames::en("dry");
        assert_eq!(localized_name(Language::ZhCN, &names), "dry");
    }

    #[test]
    fn localized_name_en_always_uses_en() {
        let names = vlabel_core::LocalizedNames {
            en: "trash".to_string(),
            zh: Some("垃圾桶".to_string()),
        };
        assert_eq!(localized_name(Language::EnUS, &names), "trash");
    }
}
