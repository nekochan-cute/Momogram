const JAVA_DECIMAL_DIGIT_ZEROES: &[u32] = &[
    0x0030, 0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6,
    0x0B66, 0x0BE6, 0x0C66, 0x0CE6, 0x0D66, 0x0DE6, 0x0E50, 0x0ED0,
    0x0F20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80,
    0x1A90, 0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0x2080, 0xA620, 0xA8D0,
    0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0, 0xFF10,
];

/// Mirrors the JVM/Kotlin `Character.digit(char.code, radix)` behavior for
/// UTF-16 `Char` values. Supplementary scalar values are rejected because a
/// Java/Kotlin String exposes them as two surrogate code units.
pub(crate) fn java_char_digit(ch: char, radix: u8) -> Option<u8> {
    if !(2..=36).contains(&radix) {
        return None;
    }

    let code = ch as u32;
    if code > u16::MAX as u32 {
        return None;
    }

    for &zero in JAVA_DECIMAL_DIGIT_ZEROES {
        if (zero..=zero + 9).contains(&code) {
            let digit = (code - zero) as u8;
            return (digit < radix).then_some(digit);
        }
    }

    let digit = match code {
        0x0041..=0x005A => (code - 0x0041 + 10) as u8,
        0x0061..=0x007A => (code - 0x0061 + 10) as u8,
        0xFF21..=0xFF3A => (code - 0xFF21 + 10) as u8,
        0xFF41..=0xFF5A => (code - 0xFF41 + 10) as u8,
        _ => return None,
    };
    (digit < radix).then_some(digit)
}

pub(crate) fn java_parse_i32(value: &str) -> Option<i32> {
    let mut chars = value.chars();
    let first = chars.next()?;

    let (negative, first_digit) = match first {
        '-' => (true, None),
        '+' => (false, None),
        ch => (false, Some(ch)),
    };

    let limit = if negative {
        i32::MAX as u64 + 1
    } else {
        i32::MAX as u64
    };
    let mut parsed = 0u64;
    let mut saw_digit = false;

    let mut consume = |ch: char| -> bool {
        let Some(digit) = java_char_digit(ch, 10) else {
            return false;
        };
        let digit = digit as u64;
        if parsed > (limit - digit) / 10 {
            return false;
        }
        parsed = parsed * 10 + digit;
        saw_digit = true;
        true
    };

    if let Some(ch) = first_digit {
        if !consume(ch) {
            return None;
        }
    }
    for ch in chars {
        if !consume(ch) {
            return None;
        }
    }
    if !saw_digit {
        return None;
    }

    if negative {
        if parsed == i32::MAX as u64 + 1 {
            Some(i32::MIN)
        } else {
            Some(-(parsed as i32))
        }
    } else {
        Some(parsed as i32)
    }
}

pub(crate) fn java_char_is_whitespace(ch: char) -> bool {
    match ch as u32 {
        0x0009..=0x000D | 0x001C..=0x001F => true,
        0x0020 | 0x1680 | 0x2000..=0x2006 | 0x2008..=0x200A | 0x2028 | 0x2029
        | 0x205F | 0x3000 => true,
        // Java deliberately excludes non-breaking spaces U+00A0, U+2007,
        // and U+202F even though they are Unicode SPACE_SEPARATOR values.
        _ => false,
    }
}

pub(crate) fn java_is_blank(value: &str) -> bool {
    value.is_empty() || value.chars().all(java_char_is_whitespace)
}

pub(crate) fn kotlin_char_is_whitespace(ch: char) -> bool {
    match ch as u32 {
        0x0009..=0x000D | 0x001C..=0x001F => true,
        0x0020 | 0x00A0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029
        | 0x202F | 0x205F | 0x3000 => true,
        _ => false,
    }
}

pub(crate) fn kotlin_is_blank(value: &str) -> bool {
    value.is_empty() || value.chars().all(kotlin_char_is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrors_java_whitespace_rules() {
        assert!(java_is_blank(" \t\n"));
        assert!(java_is_blank("\u{3000}"));
        assert!(!java_is_blank("\u{00A0}"));
        assert!(!java_is_blank("\u{2007}"));
        assert!(!java_is_blank("\u{202F}"));
        assert!(!java_is_blank("\u{0085}"));
    }

    #[test]
    fn mirrors_kotlin_whitespace_rules() {
        assert!(kotlin_is_blank(" \t\n"));
        assert!(kotlin_is_blank("\u{3000}"));
        assert!(kotlin_is_blank("\u{00A0}"));
        assert!(kotlin_is_blank("\u{2007}"));
        assert!(kotlin_is_blank("\u{202F}"));
        assert!(!kotlin_is_blank("\u{0085}"));
    }

    #[test]
    fn parses_i32_like_java_and_kotlin() {
        assert_eq!(java_parse_i32("2147483647"), Some(i32::MAX));
        assert_eq!(java_parse_i32("-2147483648"), Some(i32::MIN));
        assert_eq!(java_parse_i32("2147483648"), None);
        assert_eq!(java_parse_i32("١٢٣"), Some(123));
        assert_eq!(java_parse_i32("１２３"), Some(123));
        assert_eq!(java_parse_i32("+５"), Some(5));
        assert_eq!(java_parse_i32("+"), None);
    }

    #[test]
    fn mirrors_java_unicode_digit_examples() {
        assert_eq!(java_char_digit('9', 10), Some(9));
        assert_eq!(java_char_digit('٥', 10), Some(5));
        assert_eq!(java_char_digit('５', 10), Some(5));
        assert_eq!(java_char_digit('Ｆ', 16), Some(15));
        assert_eq!(java_char_digit('Ｆ', 10), None);
        assert_eq!(java_char_digit('𝟝', 10), None);
    }
}
