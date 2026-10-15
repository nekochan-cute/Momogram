use crate::java_compat::{java_is_blank, java_parse_i32, kotlin_is_blank};
use oxixml_unicode::case::simple_uppercase;
use once_cell::sync::Lazy;
use unicode_bidi::{bidi_class, BidiClass};
use uuid::Uuid;

const RE_KEYS: &[char] = &['$', '(', ')', '*', '+', '.', '[', ']', '?', '\\', '^', '{', '}', '|'];
const RANDOM_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

pub fn get_sub_string(
    text: &str,
    left: Option<&str>,
    right: Option<&str>,
    last: bool,
) -> Result<String, String> {
    let text16: Vec<u16> = text.encode_utf16().collect();
    let left16 = left
        .filter(|s| !s.is_empty())
        .map(|s| s.encode_utf16().collect::<Vec<_>>());
    let right16 = right
        .filter(|s| !s.is_empty())
        .map(|s| s.encode_utf16().collect::<Vec<_>>());

    let start = match left16.as_deref() {
        None => 0,
        Some(needle) => find_subslice(&text16, needle, 0, last)
            .map(|i| i + needle.len())
            .unwrap_or(0),
    };

    let end = match right16.as_deref() {
        None => text16.len(),
        Some(needle) if last => find_subslice(&text16, needle, 0, true).unwrap_or(text16.len()),
        Some(needle) => find_subslice(&text16, needle, start, false).unwrap_or(text16.len()),
    };

    if start > end {
        return Err(format!("begin {start}, end {end}, length {}", text16.len()));
    }
    Ok(String::from_utf16_lossy(&text16[start..end]))
}

fn find_subslice(haystack: &[u16], needle: &[u16], start: usize, last: bool) -> Option<usize> {
    if needle.is_empty() || start > haystack.len() || needle.len() > haystack.len() {
        return None;
    }
    if last {
        (start..=haystack.len().saturating_sub(needle.len()))
            .rev()
            .find(|&i| haystack.get(i..i + needle.len()) == Some(needle))
    } else {
        (start..=haystack.len().saturating_sub(needle.len()))
            .find(|&i| haystack.get(i..i + needle.len()) == Some(needle))
    }
}

pub fn is_integer(value: &str) -> bool {
    java_parse_i32(value).is_some()
}

pub fn replace_all_regex(content: &str, pattern: &str, replacement: Option<&str>) -> Result<String, String> {
    if content.is_empty() {
        return Ok(String::new());
    }

    // Legacy Kotlin compiles the Pattern before inspecting a nullable
    // replacement template, so invalid regex syntax must still surface.
    let regex = crate::java_regex::compile(pattern, crate::java_regex::DOTALL)?;
    let Some(template) = replacement else {
        return Ok(content.to_owned());
    };
    let mut out = String::with_capacity(content.len());
    let mut last = 0;

    for captures in regex.captures_iter(content) {
        let captures = captures.map_err(|e| e.to_string())?;
        let whole = captures
            .get(0)
            .ok_or_else(|| "regex match is missing group 0".to_owned())?;
        out.push_str(&content[last..whole.start()]);
        match expand_groups(template, &captures)? {
            Some(replacement) => out.push_str(&replacement),
            None => out.push_str(whole.as_str()),
        }
        last = whole.end();
    }
    out.push_str(&content[last..]);
    Ok(out)
}

fn expand_groups(template: &str, captures: &fancy_regex::Captures<'_>) -> Result<Option<String>, String> {
    let bytes = template.as_bytes();
    let mut tokens: Vec<&str> = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            let token = &template[start..end];
            if !tokens.contains(&token) {
                tokens.push(token);
            }
            i = end;
        } else {
            let ch = template[i..]
                .chars()
                .next()
                .ok_or_else(|| "invalid UTF-8 replacement offset".to_owned())?;
            i += ch.len_utf8();
        }
    }

    // Kotlin uses a TreeSet ordered by descending token length, then
    // descending lexical order, and String.replace for each group token.
    tokens.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| b.cmp(a)));

    let mut replacement = template.to_owned();
    for token in tokens {
        let group: usize = token
            .parse()
            .map_err(|_| "invalid replacement group".to_owned())?;
        if group >= captures.len() {
            return Err(format!("replacement group ${group} is out of range"));
        }
        let Some(value) = captures.get(group) else {
            // Legacy Kotlin makes the nullable replacement null here. The
            // safe equivalent is to leave this whole match unchanged.
            return Ok(None);
        };
        replacement = replacement.replace(&format!("${token}"), value.as_str());
    }

    Ok(Some(replacement))
}

pub fn escape(content: &str) -> String {
    if java_is_blank(content) {
        return content.to_owned();
    }
    let mut out = String::with_capacity(content.len());
    for ch in content.chars() {
        if RE_KEYS.contains(&ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

pub fn simple_uuid() -> String {
    Uuid::new_v4().simple().to_string()
}

pub fn is_rtl(value: Option<&str>) -> bool {
    value.is_some_and(|text| {
        text.chars().any(|ch| matches!(bidi_class(ch), BidiClass::R | BidiClass::AL))
    })
}

pub fn first_char_upper(value: Option<&str>) -> Option<String> {
    let value = value?;
    if java_is_blank(value) {
        return Some(value.to_owned());
    }

    // Kotlin's String[index] operates on UTF-16 code units. Preserve that
    // behavior instead of uppercasing a whole Unicode scalar value.
    let mut units: Vec<u16> = value.encode_utf16().collect();
    let first = *units.first()?;
    if !(0xD800..=0xDFFF).contains(&first) {
        if let Some(ch) = char::from_u32(first as u32) {
            let upper = simple_uppercase(ch) as u32;
            if upper <= u16::MAX as u32 {
                units[0] = upper as u16;
            }
        }
    }
    Some(String::from_utf16_lossy(&units))
}

pub fn prefix_030(simple_name: &str) -> String {
    format!("030-{simple_name}")
}


pub fn select_name(use_old: bool, old: &str, current: &str) -> String {
    if use_old { old.to_owned() } else { current.to_owned() }
}

pub fn is_app_name(value: &str, a: &str, b: &str, c: &str) -> bool {
    value == a || value == b || value == c
}

pub fn append_csv(original: Option<&str>, addition: &str) -> String {
    match original {
        None => addition.to_owned(),
        Some(value) if kotlin_is_blank(value) => addition.to_owned(),
        Some(value) => format!("{value},{addition}"),
    }
}

pub fn randomize_file_name(name: &str) -> Result<String, String> {
    let ext = name.rfind('.').map(|dot| &name[dot..]).unwrap_or("");
    let mut out = String::with_capacity(6 + ext.len());
    let acceptance_limit = (u8::MAX as usize + 1) / RANDOM_CHARS.len() * RANDOM_CHARS.len();

    while out.len() < 6 {
        let mut random = [0u8; 8];
        getrandom::getrandom(&mut random).map_err(|e| e.to_string())?;
        for byte in random {
            if (byte as usize) >= acceptance_limit {
                continue;
            }
            out.push(RANDOM_CHARS[(byte as usize) % RANDOM_CHARS.len()] as char);
            if out.len() == 6 {
                break;
            }
        }
    }

    out.push_str(ext);
    Ok(out)
}

fn android_regex_digit(ch: char) -> bool {
    static DIGIT: Lazy<Result<regex::Regex, regex::Error>> =
        Lazy::new(|| regex::Regex::new(r"^\d$"));
    let Ok(regex) = DIGIT.as_ref() else {
        return false;
    };
    let mut buf = [0u8; 4];
    regex.is_match(ch.encode_utf8(&mut buf))
}

pub fn mention_ranges(text: &str) -> Vec<i32> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut result = Vec::new();
    let mut utf16_pos = 0usize;
    let mut utf16_positions = Vec::with_capacity(chars.len() + 1);
    for &(_, ch) in &chars {
        utf16_positions.push(utf16_pos);
        utf16_pos += ch.len_utf16();
    }
    utf16_positions.push(utf16_pos);

    let mut i = 0;
    while i < chars.len() {
        if chars[i].1 != '@' {
            i += 1;
            continue;
        }
        let start = i;
        let mut end = i + 1;
        let mut count = 0;
        while end < chars.len() && count < 32 {
            let ch = chars[end].1;
            if !(ch.is_ascii_alphabetic() || android_regex_digit(ch) || ch == '_') {
                break;
            }
            end += 1;
            count += 1;
        }
        if count >= 1 {
            result.push(utf16_positions[start] as i32);
            result.push(utf16_positions[end] as i32);
            i = end;
        } else {
            i += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substring_uses_jvm_utf16_offsets() {
        assert_eq!(
            get_sub_string("😀<hello>😀", Some("<"), Some(">"), false).unwrap(),
            "hello"
        );
    }

    #[test]
    fn substring_last_mode_uses_global_right_delimiter_like_kotlin() {
        assert!(get_sub_string("x>abc<y", Some("<"), Some(">"), true).is_err());
        assert_eq!(
            get_sub_string("x<abc>y", Some("<"), Some(">"), true).unwrap(),
            "abc"
        );
    }

    #[test]
    fn append_csv_uses_kotlin_blank_semantics() {
        assert_eq!(append_csv(Some(" \t"), "x"), "x");
        assert_eq!(append_csv(Some("\u{00A0}"), "x"), "x");
        assert_eq!(append_csv(Some("\u{2007}"), "x"), "x");
        assert_eq!(append_csv(Some("\u{202F}"), "x"), "x");
        assert_eq!(append_csv(Some("\u{0085}"), "x"), "\u{0085},x");
    }

    #[test]
    fn apache_blank_call_sites_keep_java_whitespace_semantics() {
        assert_eq!(escape("\u{00A0}"), "\u{00A0}");
        assert_eq!(first_char_upper(Some("\u{00A0}a")).as_deref(), Some("\u{00A0}a"));
    }

    #[test]
    fn integer_parsing_matches_kotlin_unicode_digit_semantics() {
        assert!(is_integer("2147483647"));
        assert!(is_integer("-2147483648"));
        assert!(!is_integer("2147483648"));
        assert!(!is_integer("-2147483649"));
        assert!(is_integer("١٢٣"));
        assert!(is_integer("１２３"));
        assert!(!is_integer("𝟙"));
        assert!(!is_integer("+"));
        assert!(!is_integer(""));
    }

    #[test]
    fn null_replacement_still_validates_pattern_like_kotlin() {
        assert!(replace_all_regex("content", "(", None).is_err());
        assert_eq!(
            replace_all_regex("content", "valid", None).unwrap(),
            "content"
        );
    }

    #[test]
    fn replacement_expands_numeric_groups() {
        assert_eq!(replace_all_regex("a12b34", r"(\d+)", Some("[$1]")).unwrap(), "a[12]b[34]");
    }

    #[test]
    fn unmatched_optional_group_keeps_original_match_like_kotlin() {
        assert_eq!(
            replace_all_regex("b", r"(a)?b", Some("[$1]")).unwrap(),
            "b"
        );
    }

    #[test]
    fn unmatched_group_does_not_partially_expand_replacement() {
        assert_eq!(
            replace_all_regex("b", r"(a)?(b)", Some("$2-$1")).unwrap(),
            "b"
        );
    }

    #[test]
    fn replacement_uses_legacy_descending_group_token_order() {
        assert_eq!(
            replace_all_regex("a$1", r"(a)(\$1)", Some("$2-$1")).unwrap(),
            "a-a"
        );
    }
    #[test]
    fn mentions_report_utf16_ranges() {
        assert_eq!(mention_ranges("😀 @abc x"), vec![3, 7]);
        assert_eq!(mention_ranges("@a１２٣_ z"), vec![0, 6]);
        // A supplementary Unicode decimal digit counts as one regex character
        // but two UTF-16 code units in Android span offsets.
        assert_eq!(mention_ranges("@a𝟙_ z"), vec![0, 5]);
        assert!(mention_ranges("@é").is_empty());
    }

    #[test]
    fn random_name_preserves_trailing_dot_like_kotlin() {
        let value = randomize_file_name("photo.").unwrap();
        assert_eq!(value.len(), 7);
        assert!(value.ends_with('.'));
    }

    #[test]
    fn random_name_keeps_extension() {
        let value = randomize_file_name("photo.jpg").unwrap();
        assert_eq!(value.len(), 10);
        assert!(value.ends_with(".jpg"));
    }

    #[test]
    fn first_char_upper_matches_kotlin_utf16_char_semantics() {
        assert_eq!(first_char_upper(Some("abc")).as_deref(), Some("Abc"));
        assert_eq!(first_char_upper(Some("äbc")).as_deref(), Some("Äbc"));
        assert_eq!(first_char_upper(Some("ßeta")).as_deref(), Some("ßeta"));
        // Kotlin uppercases only str[0], the high surrogate, so a supplementary
        // lowercase letter is left unchanged rather than uppercased as a scalar.
        assert_eq!(first_char_upper(Some("𐐨bc")).as_deref(), Some("𐐨bc"));
    }
}
