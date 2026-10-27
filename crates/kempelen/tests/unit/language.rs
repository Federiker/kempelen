use super::Language;

#[test]
fn display_preserves_every_language_pack_code() {
    for language in Language::ALL {
        assert_eq!(language.to_string(), language.code());
        assert_eq!(format!("{language}"), language.code());
        assert_eq!(serde_json::to_value(language).unwrap(), language.code());
    }
}
