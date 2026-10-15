use fancy_regex::Regex;

pub(crate) const UNIX_LINES: i32 = 0x01;
pub(crate) const CASE_INSENSITIVE: i32 = 0x02;
pub(crate) const COMMENTS: i32 = 0x04;
pub(crate) const MULTILINE: i32 = 0x08;
pub(crate) const LITERAL: i32 = 0x10;
pub(crate) const DOTALL: i32 = 0x20;
pub(crate) const UNICODE_CASE: i32 = 0x40;
pub(crate) const CANON_EQ: i32 = 0x80;
pub(crate) const UNICODE_CHARACTER_CLASS: i32 = 0x100;

pub(crate) fn compile(pattern: &str, flags: i32) -> Result<Regex, String> {
    if flags & CANON_EQ != 0 {
        return Err("CANON_EQ is not supported on Android".to_owned());
    }

    // Android always uses Unicode-aware character classes and Unicode-aware
    // case folding. UNICODE_CASE and UNICODE_CHARACTER_CLASS are therefore
    // no-ops there.
    let dotall = flags & DOTALL != 0;
    let unix_lines = flags & UNIX_LINES != 0;
    let mut body = if flags & LITERAL != 0 {
        regex::escape(pattern)
    } else {
        rewrite_android_syntax(pattern, dotall, unix_lines)
    };

    let mut modifiers = String::new();
    if flags & CASE_INSENSITIVE != 0 {
        modifiers.push('i');
    }
    if flags & COMMENTS != 0 {
        modifiers.push('x');
    }
    if flags & MULTILINE != 0 {
        modifiers.push('m');
    }
    if flags & DOTALL != 0 {
        modifiers.push('s');
    }
    if !modifiers.is_empty() {
        body = format!("(?{modifiers}:{body})");
    }

    Regex::new(&body).map_err(|e| e.to_string())
}

fn rewrite_android_syntax(pattern: &str, dotall: bool, unix_lines: bool) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut i = 0usize;
    let mut class_depth = 0usize;
    let mut quoted = false;

    while i < chars.len() {
        let ch = chars[i];

        if quoted {
            if ch == '\\' && chars.get(i + 1) == Some(&'E') {
                quoted = false;
                i += 2;
                continue;
            }
            out.push_str(&regex::escape(&ch.to_string()));
            i += 1;
            continue;
        }

        if ch == '\\' {
            let Some(&next) = chars.get(i + 1) else {
                out.push('\\');
                break;
            };

            if next == 'Q' {
                quoted = true;
                i += 2;
                continue;
            }

            let in_class = class_depth > 0;
            let replacement = match next {
                // Android's Pattern always uses Unicode-aware \\d/\\s/\\w and
                // word boundaries. fancy-regex delegates those constructs to
                // regex, whose Unicode classes follow the same UTS #18 model.
                'h' => Some(if in_class {
                    " \\t\\u{00A0}\\u{1680}\\u{180E}\\u{2000}-\\u{200A}\\u{202F}\\u{205F}\\u{3000}"
                } else {
                    "[ \\t\\u{00A0}\\u{1680}\\u{180E}\\u{2000}-\\u{200A}\\u{202F}\\u{205F}\\u{3000}]"
                }),
                'H' => Some("[^ \\t\\u{00A0}\\u{1680}\\u{180E}\\u{2000}-\\u{200A}\\u{202F}\\u{205F}\\u{3000}]"),
                'v' => Some(if in_class {
                    "\\n\\x0B\\x0C\\r\\u{0085}\\u{2028}\\u{2029}"
                } else {
                    "[\\n\\x0B\\x0C\\r\\u{0085}\\u{2028}\\u{2029}]"
                }),
                'V' => Some("[^\\n\\x0B\\x0C\\r\\u{0085}\\u{2028}\\u{2029}]"),
                'b' if in_class => Some("\\x08"),
                'R' if !in_class => Some("(?:\\r\\n|[\\n\\x0B\\x0C\\r\\u{0085}\\u{2028}\\u{2029}])"),
                _ => None,
            };

            if let Some(replacement) = replacement {
                out.push_str(replacement);
            } else {
                out.push('\\');
                out.push(next);
            }
            i += 2;
            continue;
        }

        if ch == '[' {
            class_depth += 1;
            out.push(ch);
            i += 1;
            continue;
        }
        if ch == ']' && class_depth > 0 {
            class_depth -= 1;
            out.push(ch);
            i += 1;
            continue;
        }

        if ch == '.' && class_depth == 0 && !dotall {
            if unix_lines {
                out.push_str("[^\\n]");
            } else {
                out.push_str("[^\\n\\x0B\\x0C\\r\\u{0085}\\u{2028}\\u{2029}]");
            }
        } else {
            out.push(ch);
        }
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_predefined_classes_are_unicode_aware() {
        let digits = compile(r"^\d+$", 0).unwrap();
        assert!(digits.is_match("123").unwrap());
        assert!(digits.is_match("１２３").unwrap());

        let words = compile(r"^\w+$", 0).unwrap();
        assert!(words.is_match("abc_123").unwrap());
        assert!(words.is_match("é").unwrap());

        let spaces = compile(r"^\s+$", 0).unwrap();
        assert!(spaces.is_match(" \t\r\n").unwrap());
        assert!(spaces.is_match("\u{00A0}").unwrap());
    }

    #[test]
    fn android_default_dot_excludes_all_line_terminators() {
        let dot = compile(r"^.$", 0).unwrap();
        assert!(dot.is_match("x").unwrap());
        for line in ["\n", "\u{000B}", "\u{000C}", "\r", "\u{0085}", "\u{2028}", "\u{2029}"] {
            assert!(!dot.is_match(line).unwrap(), "{line:?}");
        }

        let dotall = compile(r"^.$", DOTALL).unwrap();
        assert!(dotall.is_match("\u{2028}").unwrap());

        let unix = compile(r"^.$", UNIX_LINES).unwrap();
        assert!(unix.is_match("\r").unwrap());
        assert!(!unix.is_match("\n").unwrap());
    }

    #[test]
    fn android_case_insensitive_is_unicode_aware_without_unicode_case() {
        let regex = compile("ä", CASE_INSENSITIVE).unwrap();
        assert!(regex.is_match("Ä").unwrap());

        let same = compile("ä", CASE_INSENSITIVE | UNICODE_CASE | UNICODE_CHARACTER_CLASS).unwrap();
        assert!(same.is_match("Ä").unwrap());
    }

    #[test]
    fn android_quote_escape_is_literal() {
        let quoted = compile(r"\Qa.b[0]\E", 0).unwrap();
        assert!(quoted.is_match("a.b[0]").unwrap());
        assert!(!quoted.is_match("axb0").unwrap());
    }

    #[test]
    fn android_extra_whitespace_and_linebreak_constructs_work() {
        assert!(compile(r"^\h$", 0).unwrap().is_match("\u{00A0}").unwrap());
        assert!(compile(r"^\v$", 0).unwrap().is_match("\u{2028}").unwrap());
        assert!(compile(r"^\R$", 0).unwrap().is_match("\r\n").unwrap());
    }

    #[test]
    fn canon_eq_is_rejected_like_android() {
        assert!(compile("abc", CANON_EQ).is_err());
    }
}
