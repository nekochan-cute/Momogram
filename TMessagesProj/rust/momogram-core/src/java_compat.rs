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

#[cfg(test)]
mod tests {
    use super::*;

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
