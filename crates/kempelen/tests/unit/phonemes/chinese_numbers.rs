use super::spell_numbers;

#[test]
fn plain_numbers_are_read_as_amounts() {
    for (written, spoken) in [
        ("0", "零"),
        ("10", "十"),
        ("12", "十二"),
        ("105", "一百零五"),
        ("320", "三百二十"),
        ("1010", "一千零一十"),
        ("10000", "一万"),
        ("20000", "二万"),
        ("1000000", "一百万"),
        ("49.8", "四十九点八"),
        ("3.5", "三点五"),
        ("0.5", "零点五"),
        ("-5", "负五"),
    ] {
        assert_eq!(spell_numbers(written), spoken, "{written}");
    }
}

#[test]
fn numbers_inside_sentences_keep_their_surroundings() {
    assert_eq!(spell_numbers("我有3个苹果。"), "我有三个苹果。");
    assert_eq!(spell_numbers("没有数字"), "没有数字");
    assert_eq!(spell_numbers("共1,234页"), "共一千二百三十四页");
    assert_eq!(spell_numbers("共１２页"), "共十二页");
}

#[test]
fn dates_fractions_percentages_and_temperatures_have_their_own_readings() {
    assert_eq!(spell_numbers("2024年5月12日"), "二零二四年五月十二日");
    assert_eq!(spell_numbers("12日"), "十二日");
    assert_eq!(spell_numbers("2/3"), "三分之二");
    assert_eq!(spell_numbers("50%"), "百分之五十");
    assert_eq!(spell_numbers("25℃"), "二十五摄氏度");
    assert_eq!(spell_numbers("1-2斤"), "一到二斤");
}

#[test]
fn identifiers_are_read_digit_by_digit_and_hyphens_between_numbers_are_ranges() {
    assert_eq!(spell_numbers("007"), "零零七");
    assert_eq!(spell_numbers("13800138000"), "一三八零零一三八零零零");
    assert_eq!(spell_numbers("2020-2021"), "二千零二十到二千零二十一");
    assert_eq!(spell_numbers("第3章"), "第三章");
}
