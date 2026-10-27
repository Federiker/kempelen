use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
/// A book language: what a text is written in, which decides how it is pronounced (see
/// [`crate::phonemes::reading_code`]). Its serialised name is its code (`en`, `zh-TW`, `sr-Latn`).
pub enum Language {
    #[serde(rename = "en")]
    #[default]
    English,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "cy")]
    Welsh,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "nl")]
    Dutch,
    #[serde(rename = "pl")]
    Polish,
    #[serde(rename = "pt")]
    Portuguese,
    #[serde(rename = "ru")]
    Russian,
    #[serde(rename = "el")]
    Greek,
    #[serde(rename = "da")]
    Danish,
    #[serde(rename = "fi")]
    Finnish,
    #[serde(rename = "is")]
    Icelandic,
    #[serde(rename = "no")]
    Norwegian,
    #[serde(rename = "sv")]
    Swedish,
    #[serde(rename = "zh")]
    Chinese,
    /// Traditional characters as written in Taiwan.
    #[serde(rename = "zh-TW")]
    ChineseTaiwan,
    /// Traditional characters as written in Hong Kong.
    #[serde(rename = "zh-HK")]
    ChineseHongKong,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "ca")]
    Catalan,
    #[serde(rename = "ro")]
    Romanian,
    #[serde(rename = "cs")]
    Czech,
    #[serde(rename = "sk")]
    Slovak,
    #[serde(rename = "hr")]
    Croatian,
    #[serde(rename = "sl")]
    Slovenian,
    #[serde(rename = "hu")]
    Hungarian,
    #[serde(rename = "tr")]
    Turkish,
    #[serde(rename = "et")]
    Estonian,
    #[serde(rename = "lv")]
    Latvian,
    #[serde(rename = "lt")]
    Lithuanian,
    #[serde(rename = "af")]
    Afrikaans,
    #[serde(rename = "sq")]
    Albanian,
    #[serde(rename = "id")]
    Indonesian,
    #[serde(rename = "eu")]
    Basque,
    #[serde(rename = "ga")]
    Irish,
    #[serde(rename = "sw")]
    Swahili,
    #[serde(rename = "rw")]
    Kinyarwanda,
    #[serde(rename = "rn")]
    Kirundi,
    #[serde(rename = "so")]
    Somali,
    #[serde(rename = "om")]
    Oromo,
    #[serde(rename = "lg")]
    Luganda,
    #[serde(rename = "ln")]
    Lingala,
    #[serde(rename = "sn")]
    Shona,
    #[serde(rename = "uk")]
    Ukrainian,
    #[serde(rename = "bg")]
    Bulgarian,
    #[serde(rename = "mk")]
    Macedonian,
    #[serde(rename = "sr")]
    SerbianCyrillic,
    #[serde(rename = "sr-Latn")]
    SerbianLatin,
    #[serde(rename = "ms")]
    Malay,
    #[serde(rename = "yo")]
    Yoruba,
    #[serde(rename = "ig")]
    Igbo,
    #[serde(rename = "ha")]
    Hausa,
    #[serde(rename = "ee")]
    Ewe,
    #[serde(rename = "ak")]
    Akan,
}

impl Language {
    /// Every language, Japanese, Ukrainian and Bulgarian included, which narration cannot read yet
    /// ([`crate::phonemes::can_narrate`]).
    pub const ALL: [Language; 55] = [
        Language::English,
        Language::Italian,
        Language::Welsh,
        Language::German,
        Language::Spanish,
        Language::French,
        Language::Dutch,
        Language::Polish,
        Language::Portuguese,
        Language::Russian,
        Language::Greek,
        Language::Danish,
        Language::Finnish,
        Language::Icelandic,
        Language::Norwegian,
        Language::Swedish,
        Language::Chinese,
        Language::ChineseTaiwan,
        Language::ChineseHongKong,
        Language::Japanese,
        Language::Catalan,
        Language::Romanian,
        Language::Czech,
        Language::Slovak,
        Language::Croatian,
        Language::Slovenian,
        Language::Hungarian,
        Language::Turkish,
        Language::Estonian,
        Language::Latvian,
        Language::Lithuanian,
        Language::Afrikaans,
        Language::Albanian,
        Language::Indonesian,
        Language::Basque,
        Language::Irish,
        Language::Swahili,
        Language::Kinyarwanda,
        Language::Kirundi,
        Language::Somali,
        Language::Oromo,
        Language::Luganda,
        Language::Lingala,
        Language::Shona,
        Language::Ukrainian,
        Language::Bulgarian,
        Language::Macedonian,
        Language::SerbianCyrillic,
        Language::SerbianLatin,
        Language::Malay,
        Language::Yoruba,
        Language::Igbo,
        Language::Hausa,
        Language::Ewe,
        Language::Akan,
    ];

    /// The language's code: two letters, or `zh-TW`, `zh-HK` and `sr-Latn`.
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Italian => "it",
            Language::Welsh => "cy",
            Language::German => "de",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::Dutch => "nl",
            Language::Polish => "pl",
            Language::Portuguese => "pt",
            Language::Russian => "ru",
            Language::Greek => "el",
            Language::Danish => "da",
            Language::Finnish => "fi",
            Language::Icelandic => "is",
            Language::Norwegian => "no",
            Language::Swedish => "sv",
            Language::Chinese => "zh",
            Language::ChineseTaiwan => "zh-TW",
            Language::ChineseHongKong => "zh-HK",
            Language::Japanese => "ja",
            Language::Catalan => "ca",
            Language::Romanian => "ro",
            Language::Czech => "cs",
            Language::Slovak => "sk",
            Language::Croatian => "hr",
            Language::Slovenian => "sl",
            Language::Hungarian => "hu",
            Language::Turkish => "tr",
            Language::Estonian => "et",
            Language::Latvian => "lv",
            Language::Lithuanian => "lt",
            Language::Afrikaans => "af",
            Language::Albanian => "sq",
            Language::Indonesian => "id",
            Language::Basque => "eu",
            Language::Irish => "ga",
            Language::Swahili => "sw",
            Language::Kinyarwanda => "rw",
            Language::Kirundi => "rn",
            Language::Somali => "so",
            Language::Oromo => "om",
            Language::Luganda => "lg",
            Language::Lingala => "ln",
            Language::Shona => "sn",
            Language::Ukrainian => "uk",
            Language::Bulgarian => "bg",
            Language::Macedonian => "mk",
            Language::SerbianCyrillic => "sr",
            Language::SerbianLatin => "sr-Latn",
            Language::Malay => "ms",
            Language::Yoruba => "yo",
            Language::Igbo => "ig",
            Language::Hausa => "ha",
            Language::Ewe => "ee",
            Language::Akan => "ak",
        }
    }

    /// Chinese in any script variant: Simplified, or Traditional as written in Taiwan or Hong Kong.
    pub fn is_chinese(&self) -> bool {
        matches!(
            self,
            Language::Chinese | Language::ChineseTaiwan | Language::ChineseHongKong
        )
    }
}

impl std::fmt::Display for Language {
    /// The language's code (`en`, `it`, `cy`…).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

#[cfg(test)]
#[path = "../tests/unit/language.rs"]
mod tests;
