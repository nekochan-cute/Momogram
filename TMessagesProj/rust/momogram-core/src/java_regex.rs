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
    if flags & (UNIX_LINES | CANON_EQ | UNICODE_CHARACTER_CLASS) != 0 {
        return Err("Java Pattern flags are not supported by the Rust compatibility layer".to_owned());
    }
    if flags & CASE_INSENSITIVE != 0 && flags & UNICODE_CASE == 0 {
        return Err("ASCII-only Java CASE_INSENSITIVE is not supported".to_owned());
    }

    let mut body = if flags & LITERAL != 0 {
        regex::escape(pattern)
    } else {
        rewrite_default_java_syntax(pattern, flags & DOTALL != 0)
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

fn rewrite_default_java_syntax(pattern: &str, dotall: bool) -> String {
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
                'd' => Some(if in_class { "0-9" } else { "[0-9]" }),
                'D' => Some("[^0-9]"),
                's' => Some(if in_class { " \\t\\n\\x0B\\x0C\\r" } else { "[ \\t\\n\\x0B\\x0C\\r]" }),
                'S' => Some("[^ \\t\\n\\x0B\\x0C\\r]"),
                'w' => Some(if in_class { "A-Za-z0-9_" } else { "[A-Za-z0-9_]" }),
                'W' => Some("[^A-Za-z0-9_]"),
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
                'b' => Some("(?:(?<![A-Za-z0-9_])(?=[A-Za-z0-9_])|(?<=[A-Za-z0-9_])(?![A-Za-z0-9_]))"),
                'B' if !in_class => Some("(?:(?<=[A-Za-z0-9_])(?=[A-Za-z0-9_])|(?<![A-Za-z0-9_])(?![A-Za-z0-9_]))"),
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
            out.push_str("[^\\n\\r\\u{0085}\\u{2028}\\u{2029}]");
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
    fn java_default_perl_classes_are_ascii() {
        let digits = compile(r"^\d+$", 0).unwrap();
        assert!(digits.is_match("123").unwrap());
        assert!(!digits.is_match("１２３").unwrap());

        let words = compile(r"^\w+$", 0).unwrap();
        assert!(words.is_match("abc_123").unwrap());
        assert!(!words.is_match("é").unwrap());

        let spaces = compile(r"^\s+$", 0).unwrap();
        assert!(spaces.is_match(" \t\r\n").unwrap());
        assert!(!spaces.is_match("\u{00A0}").unwrap());
    }

    #[test]
    fn java_default_dot_excludes_all_java_line_terminators() {
        let dot = compile(r"^.$", 0).unwrap();
        assert!(dot.is_match("x").unwrap());
        assert!(!dot.is_match("\r").unwrap());
        assert!(!dot.is_match("\u{0085}").unwrap());
        assert!(!dot.is_match("\u{2028}").unwrap());
        assert!(!dot.is_match("\u{2029}").unwrap());

        let dotall = compile(r"^.$", DOTALL).unwrap();
        assert!(dotall.is_match("\u{2028}").unwrap());
    }

    #[test]
    fn java_quote_escape_is_literal() {
        let quoted = compile(r"\Qa.b[0]\E", 0).unwrap();
        assert!(quoted.is_match("a.b[0]").unwrap());
        assert!(!quoted.is_match("axb0").unwrap());
    }

    #[test]
    fn java_classes_work_inside_character_classes() {
        let digits = compile(r"^[\d_]+$", 0).unwrap();
        assert!(digits.is_match("1_2").unwrap());
        assert!(!digits.is_match("１").unwrap());
    }

    #[test]
    fn java_word_boundaries_are_ascii() {
        let boundary = compile(r"\bé\b", 0).unwrap();
        assert!(!boundary.is_match("é").unwrap());
        let ascii = compile(r"\ba\b", 0).unwrap();
        assert!(ascii.is_match("a").unwrap());
    }
}
