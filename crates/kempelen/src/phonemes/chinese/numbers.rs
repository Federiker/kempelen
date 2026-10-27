//! Spelling Arabic numerals in Chinese, as Kokoro's Chinese voices were trained to hear them.
//!
//! Follows cn2an's `an2cn` conversion (MIT) that misaki applies before pronunciation: dates,
//! fractions, percentages, temperatures and plain numbers become Chinese numerals. Two
//! deliberate differences keep strings of digits that are identifiers from being read as
//! amounts: runs with a leading zero or of 11 or more digits are read digit by digit, and a
//! hyphen between digits reads as a range ("到") instead of a minus sign.

#![expect(
    clippy::expect_used,
    reason = "fixed patterns and digits, all exercised by the unit tests"
)]

use regex::{Captures, Regex};
use std::sync::LazyLock;

const DIGITS: [char; 10] = ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
const UNITS: [&str; 16] = [
    "", "十", "百", "千", "万", "十", "百", "千", "亿", "十", "百", "千", "万", "十", "百", "千",
];
const MEASURE_WORDS: &str = "斤|克|千克|公斤|吨|米|厘米|毫米|公里|升|毫升|元|角|分|个|只|条|张|块|瓶|杯|份|本|辆|台|匹|头|位|亩|小时|分钟|秒|天|半";

static RANGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(\d+(?:\.\d+)?)-(\d+(?:\.\d+)?)({MEASURE_WORDS})"
    ))
    .expect("valid pattern")
});
static DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:\d{2,4}年)?(?:\d{1,2}月)?(?:\d{1,2}日)?").expect("valid pattern")
});
static YEAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)年").expect("valid pattern"));
static FRACTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+)/(\d+)").expect("valid pattern"));
static PERCENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(-?(?:\d+\.)?\d+)%").expect("valid pattern"));
static CELSIUS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)℃").expect("valid pattern"));
static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-?(?:\d+\.)?\d+").expect("valid pattern"));
static GROUPED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d),(\d{3})").expect("valid pattern"));
static ANY_DIGIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+").expect("valid pattern"));
static ZERO_BEFORE_THOUSANDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([万亿])零([一二三四五六七八九][千])").expect("valid pattern"));

/// Replace every Arabic number in `text` with its Chinese reading.
pub(super) fn spell_numbers(text: &str) -> String {
    let text: String = text
        .chars()
        .map(|character| match character {
            '０'..='９' => char::from_u32(character as u32 - '０' as u32 + '0' as u32)
                .expect("fullwidth digits map to ASCII digits"),
            other => other,
        })
        .collect();
    if !text.chars().any(|character| character.is_ascii_digit()) {
        return text;
    }
    let mut text = text;
    while GROUPED.is_match(&text) {
        text = GROUPED.replace_all(&text, "$1$2").into_owned();
    }
    let text = RANGE.replace_all(&text, |caps: &Captures| {
        format!("{}到{}{}", low(&caps[1]), low(&caps[2]), &caps[3])
    });
    let text = DATE.replace_all(&text, |caps: &Captures| {
        let whole = &caps[0];
        if whole.is_empty() {
            return String::new();
        }
        let years = YEAR.replace_all(whole, |year: &Captures| format!("{}年", direct(&year[1])));
        ANY_DIGIT
            .replace_all(&years, |digits: &Captures| low(&digits[0]))
            .into_owned()
    });
    let text = FRACTION.replace_all(&text, |caps: &Captures| {
        format!("{}分之{}", low(&caps[2]), low(&caps[1]))
    });
    let text = PERCENT.replace_all(&text, |caps: &Captures| format!("百分之{}", low(&caps[1])));
    let text = CELSIUS.replace_all(&text, |caps: &Captures| format!("{}摄氏度", low(&caps[1])));
    replace_numbers(&text)
}

fn replace_numbers(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for found in NUMBER.find_iter(text) {
        out.push_str(&text[last..found.start()]);
        last = found.end();
        let literal = found.as_str();
        match literal.strip_prefix('-') {
            // A hyphen after a digit joins two numbers ("2020-2021"); it is not a minus sign.
            Some(rest) if text[..found.start()].ends_with(|c: char| c.is_ascii_digit()) => {
                out.push('到');
                out.push_str(&low(rest));
            }
            _ => out.push_str(&low(literal)),
        }
    }
    out.push_str(&text[last..]);
    out
}

/// Read a (possibly signed, possibly decimal) number the way `an2cn` does.
fn low(number: &str) -> String {
    let (sign, number) = match number.strip_prefix('-') {
        Some(rest) => ("负", rest),
        None => ("", number),
    };
    let (whole, fraction) = match number.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (number, None),
    };
    let identifier = (whole.len() > 1 && whole.starts_with('0')) || whole.len() >= 11;
    if identifier || whole.len() > UNITS.len() {
        return format!("{sign}{}", direct(number));
    }
    let mut out = format!("{sign}{}", integer(whole));
    if let Some(fraction) = fraction {
        out.push('点');
        out.extend(fraction.chars().take(16).map(digit));
    }
    out
}

/// Read the digits one by one, as for years and identifiers.
fn direct(number: &str) -> String {
    number
        .chars()
        .map(|character| {
            if character == '.' {
                '点'
            } else {
                digit(character)
            }
        })
        .collect()
}

fn digit(character: char) -> char {
    character
        .to_digit(10)
        .map_or(character, |value| DIGITS[value as usize])
}

fn integer(digits: &str) -> String {
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return "零".to_string();
    }
    let length = digits.len();
    let mut out = String::new();
    for (index, character) in digits.chars().enumerate() {
        let place = length - index - 1;
        let value = character.to_digit(10).expect("only digits reach here") as usize;
        if value != 0 {
            out.push(DIGITS[value]);
            out.push_str(UNITS[place]);
        } else {
            if place.is_multiple_of(4) {
                out.push('零');
                out.push_str(UNITS[place]);
            }
            if index > 0 && !out.ends_with('零') {
                out.push('零');
            }
        }
    }
    let out = out
        .replace("零零", "零")
        .replace("零万", "万")
        .replace("零亿", "亿")
        .replace("亿万", "亿");
    let out = out.trim_matches('零');
    let out = ZERO_BEFORE_THOUSANDS.replace_all(out, "$1$2");
    let out = out
        .strip_prefix("一十")
        .map_or_else(|| out.to_string(), |rest| format!("十{rest}"));
    if out.is_empty() {
        "零".to_string()
    } else {
        out
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/phonemes/chinese_numbers.rs"]
mod tests;
