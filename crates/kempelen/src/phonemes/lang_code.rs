use crate::{Error, Language, Voice};

/// Return the reading code a voice reads its own language with (`en-us`, `en`, `cmn`…).
///
/// Reading codes name a pronunciation: jespersen's language (see `JespersenPhonemizer`), or the
/// Mandarin front end for `cmn`. They began as eSpeak NG's codes and never change, because
/// applications may store them (manuzio fingerprints resumed audiobooks with them).
pub fn get_lang_code(voice: Voice) -> &'static str {
    match voice {
        Voice::AfHeart
        | Voice::AfAlloy
        | Voice::AfAoede
        | Voice::AfBella
        | Voice::AfJessica
        | Voice::AfKore
        | Voice::AfNicole
        | Voice::AfNova
        | Voice::AfRiver
        | Voice::AfSarah
        | Voice::AfSky
        | Voice::AmAdam
        | Voice::AmEcho
        | Voice::AmEric
        | Voice::AmFenrir
        | Voice::AmLiam
        | Voice::AmMichael
        | Voice::AmOnyx
        | Voice::AmPuck
        | Voice::AmSanta => "en-us",
        Voice::BfAlice
        | Voice::BfEmma
        | Voice::BfIsabella
        | Voice::BfLily
        | Voice::BmDaniel
        | Voice::BmFable
        | Voice::BmGeorge
        | Voice::BmLewis => "en",
        Voice::ZfXiaobei
        | Voice::ZfXiaoni
        | Voice::ZfXiaoxiao
        | Voice::ZfXiaoyi
        | Voice::ZmYunjian
        | Voice::ZmYunxi
        | Voice::ZmYunxia
        | Voice::ZmYunyang => "cmn",
        Voice::EfDora | Voice::EmAlex | Voice::EmSanta => "es",
        Voice::FfSiwis => "fr",
        Voice::IfSara | Voice::ImNicola => "it",
        Voice::PfDora | Voice::PmAlex | Voice::PmSanta => "pt",
    }
}

/// Language a voice was trained to speak.
pub fn voice_language(voice: Voice) -> Language {
    match get_lang_code(voice) {
        "cmn" => Language::Chinese,
        "es" => Language::Spanish,
        "fr" => Language::French,
        "it" => Language::Italian,
        "pt" => Language::Portuguese,
        _ => Language::English,
    }
}

/// Reading code used to pronounce `language` text with `voice_id`.
///
/// A voice reading its own language keeps its usual pronunciation (American or
/// British English, for instance). Any other voice reads with the book language's
/// pronunciation and keeps its own timbre and accent. `None` when narration
/// cannot pronounce the language: jespersen does not read Japanese, Ukrainian or
/// Bulgarian yet (docs/more-languages.md).
pub fn reading_code(voice: Voice, language: Language) -> Option<&'static str> {
    if voice_language(voice) == language {
        return Some(get_lang_code(voice));
    }
    Some(match language {
        Language::English => "en-us",
        Language::Chinese | Language::ChineseTaiwan | Language::ChineseHongKong => "cmn",
        Language::Japanese | Language::Ukrainian | Language::Bulgarian => return None,
        Language::Greek => "el",
        Language::Swahili => "sw",
        Language::Kinyarwanda => "rw",
        Language::Kirundi => "rn",
        Language::Somali => "so",
        Language::Oromo => "om",
        Language::Luganda => "lg",
        Language::Lingala => "ln",
        Language::Shona => "sn",
        Language::Macedonian => "mk",
        Language::SerbianCyrillic => "sr",
        Language::SerbianLatin => "sr-latn",
        Language::Malay => "ms",
        Language::Yoruba => "yo",
        Language::Igbo => "ig",
        Language::Hausa => "ha",
        Language::Ewe => "ee",
        Language::Akan => "ak",
        Language::Italian => "it",
        Language::Welsh => "cy",
        Language::German => "de",
        Language::Spanish => "es",
        Language::French => "fr",
        Language::Dutch => "nl",
        Language::Polish => "pl",
        Language::Portuguese => "pt",
        Language::Russian => "ru",
        Language::Danish => "da",
        Language::Finnish => "fi",
        Language::Icelandic => "is",
        Language::Norwegian => "no",
        Language::Swedish => "sv",
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
    })
}

/// Whether narration can pronounce text written in `language`.
pub fn can_narrate(language: Language) -> bool {
    reading_code(Voice::default(), language).is_some()
}

/// The error for a book language narration cannot pronounce.
pub(crate) fn unsupported_language(language: Language) -> Error {
    Error::UnsupportedLanguage(language)
}

#[cfg(test)]
#[path = "../../tests/unit/phonemes/lang_code.rs"]
mod tests;
