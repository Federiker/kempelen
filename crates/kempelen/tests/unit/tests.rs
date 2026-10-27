#![allow(clippy::too_many_lines, reason = "one sentence per narrated language")]

use super::*;
use crate::Language;
use crate::Voice;
use crustytts_core::Tokenizer;

#[test]
fn every_narrated_language_yields_only_kokoro_tokens() {
    let samples = [
        (
            Language::German,
            "Größere Bücher, und ich möchte acht Brötchen.",
        ),
        (Language::Welsh, "Mae'r llyfr yn ddiddorol iawn."),
        (Language::Dutch, "De kleine jongen loopt naar huis."),
        (
            Language::Polish,
            "Wszystko się zmieniło, gdy przyszła zima.",
        ),
        (Language::Russian, "Мама мыла раму, и дела идут хорошо."),
        (Language::Danish, "Det er en dejlig dag i København."),
        (Language::Finnish, "Hyvää huomenta, mitä kuuluu tänään?"),
        (Language::Icelandic, "Góðan daginn, hvernig hefur þú það?"),
        (Language::Norwegian, "God morgen, hvordan har du det i dag?"),
        (Language::Swedish, "God morgon, hur mår du idag?"),
        (Language::Italian, "Il ragazzo è andato al mare."),
        (Language::English, "The quick brown fox jumps."),
        (
            Language::Catalan,
            "El llibre explica una història interessant.",
        ),
        (Language::Romanian, "Cartea spune o poveste interesantă."),
        (Language::Czech, "Kniha vypráví zajímavý příběh."),
        (Language::Slovak, "Kniha rozpráva zaujímavý príbeh."),
        (Language::Croatian, "Knjiga govori zanimljivu priču."),
        (Language::Slovenian, "Knjiga pripoveduje zanimivo zgodbo."),
        (Language::Hungarian, "A könyv érdekes történetet mesél."),
        (Language::Turkish, "Kitap ilginç bir hikâye anlatıyor."),
        (Language::Estonian, "Raamat jutustab huvitava loo."),
        (Language::Latvian, "Grāmata stāsta interesantu stāstu."),
        (Language::Lithuanian, "Knyga pasakoja įdomią istoriją."),
        (
            Language::Afrikaans,
            "Die boek vertel 'n interessante verhaal.",
        ),
        (Language::Albanian, "Libri tregon një histori interesante."),
        (
            Language::Indonesian,
            "Buku ini menceritakan kisah yang menarik.",
        ),
        (
            Language::Basque,
            "Liburuak istorio interesgarri bat kontatzen du.",
        ),
        (Language::Irish, "Insíonn an leabhar scéal suimiúil."),
        (Language::Greek, "Κάθε πρωί ο γέρος περπατά ως το ποτάμι."),
        (Language::Swahili, "Kila asubuhi mzee hutembea hadi mtoni."),
        (
            Language::Kinyarwanda,
            "Buri gitondo umusaza agenda ku mugezi.",
        ),
        (Language::Kirundi, "Buri gitondo umutama aja ku ruzi."),
        (
            Language::Somali,
            "Subax kasta odayga wuxuu u socdaa webiga.",
        ),
        (
            Language::Oromo,
            "Ganama ganama jaarsichi gara lagaatti deema.",
        ),
        (Language::Luganda, "Buli nkya omusajja omukadde atambula."),
        (
            Language::Lingala,
            "Ntɔngɔ nyonso mobange atambolaka tii na ebale.",
        ),
        (Language::Shona, "Mangwanani ese murume mukuru anofamba."),
        (Language::Macedonian, "Секое утро старецот оди до реката."),
        (
            Language::SerbianCyrillic,
            "Сваког јутра старац шета до реке.",
        ),
        (Language::SerbianLatin, "Svakog jutra starac šeta do reke."),
        (
            Language::Malay,
            "Setiap pagi, lelaki tua itu berjalan ke sungai.",
        ),
        (
            Language::Yoruba,
            "Ní àràárọ̀, arúgbó náà máa ń rìn lọ sí odò.",
        ),
        (
            Language::Igbo,
            "Kwa ụtụtụ, agadi nwoke ahụ na-aga n'osimiri.",
        ),
        (
            Language::Hausa,
            "Kowace safiya tsohon mutumin yana tafiya zuwa kogi.",
        ),
        (
            Language::Ewe,
            "Ŋdi sia ŋdi la, ŋutsu tsitsia zɔna yia tɔsisi la to.",
        ),
        (
            Language::Akan,
            "Anɔpa biara akɔkoraa no nantew kɔ asubɔnten no ho.",
        ),
    ];
    let tokenizer = crustytts_tokenizer::KokoroTokenizer::new();
    for (language, text) in samples {
        let phonemes = phonemes::get_phonemes_in(text, Voice::IfSara, language).unwrap();
        assert!(
            tokenizer.encode(&phonemes).len() > 4,
            "{language:?}: {phonemes}"
        );
        let unknown = tokenizer.unknown_chars(&phonemes);
        assert!(
            unknown.is_empty(),
            "{language:?}: {unknown:?} in {phonemes}"
        );
    }
}

#[test]
fn a_foreign_voice_uses_the_book_language_pronunciation() {
    let italian = phonemes::get_phonemes_in("Buongiorno", Voice::BfAlice, Language::Italian);
    let english = phonemes::get_phonemes("Buongiorno", Voice::BfAlice).unwrap();
    assert_eq!(
        italian.unwrap(),
        phonemes::get_phonemes("Buongiorno", Voice::IfSara).unwrap()
    );
    assert_ne!(
        phonemes::get_phonemes("Buongiorno", Voice::IfSara).unwrap(),
        english
    );
}

#[test]
fn japanese_ukrainian_and_bulgarian_text_is_refused() {
    for language in [Language::Japanese, Language::Ukrainian, Language::Bulgarian] {
        let error = phonemes::get_phonemes_in("text", Voice::BfAlice, language).unwrap_err();
        assert!(error.to_string().contains("not available"), "{error}");
    }
}
#[test]
fn test_get_phonemes_en_us() {
    let phonemes = phonemes::get_phonemes("Hello there!", Voice::AfAlloy).unwrap();
    assert_eq!(phonemes, "həlˈoʊ ðˈɛɹ!");
}

#[test]
fn test_get_phonemes_en_gb() {
    let phonemes = phonemes::get_phonemes("Hello there!", Voice::BfAlice).unwrap();
    assert_eq!(phonemes, "həlˈəʊ ðˈeə!");
}

#[test]
fn quotes_parentheses_and_dashes_survive_phonemization() {
    let phonemes = phonemes::get_phonemes(
        "\"Yes,\" she said (twice) \u{2014} wait\u{2026}",
        Voice::AfAlloy,
    )
    .unwrap();
    for mark in ['"', ',', '(', ')', '\u{2014}', '\u{2026}'] {
        assert!(phonemes.contains(mark), "{mark:?} missing from {phonemes}");
    }
    assert!(phonemes.starts_with('"') && !phonemes.starts_with("\" "));
}

#[test]
fn typographic_quotes_are_kept_as_kokoro_quotes() {
    let phonemes =
        phonemes::get_phonemes("\u{201C}Hello,\u{201D} he said.", Voice::AfAlloy).unwrap();
    assert!(phonemes.contains('\u{201C}') && phonemes.contains('\u{201D}'));
    assert!(phonemes.ends_with('.'));
}

#[test]
fn curly_apostrophes_do_not_split_words() {
    let curly = phonemes::get_phonemes("it\u{2019}s here", Voice::AfAlloy).unwrap();
    let ascii = phonemes::get_phonemes("it's here", Voice::AfAlloy).unwrap();
    assert_eq!(curly, ascii);
}

#[test]
fn every_phoneme_has_a_kokoro_token() {
    let text = "\u{00AB}Ciao,\u{00BB} disse \u{2014} \u{201C}non lo so\u{201D}\u{2026} (forse).";
    let phonemes = phonemes::get_phonemes(text, Voice::IfSara).unwrap();
    let unknown = crustytts_tokenizer::KokoroTokenizer::new().unknown_chars(&phonemes);
    assert!(unknown.is_empty(), "{unknown:?} in {phonemes}");
}

#[test]
fn numbers_keep_their_separators() {
    let phonemes = phonemes::get_phonemes("It costs 3.5 dollars.", Voice::AfAlloy).unwrap();
    // The decimal point is read ("three point five"), not taken for the end of a sentence.
    assert_eq!(phonemes.matches('.').count(), 1, "{phonemes}");
    let three = phonemes.find("θɹˈiː").expect(&phonemes);
    let point = phonemes.find("pˈɔɪnt").expect(&phonemes);
    let five = phonemes.find("fˈaɪv").expect(&phonemes);
    assert!(three < point && point < five, "{phonemes}");
}
