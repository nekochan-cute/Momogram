use crate::opencc;
use fancy_regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

pub const RESULT_BLOCKED: i32 = 0;
pub const RESULT_PASSED: i32 = 1;
pub const RESULT_PENDING: i32 = 2;
pub const RESULT_ARCHIVE: i32 = 3;
pub const RESULT_LOAD_FULL: i32 = 4;

static PENDING_IDS: Lazy<Mutex<HashMap<i64, i32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn precheck(
    ignore_blocked: bool,
    is_blocked: bool,
    auto_archive: bool,
    sender_id: i64,
    chat_id: i64,
    is_from_user: bool,
) -> i32 {
    if ignore_blocked && is_blocked {
        return RESULT_BLOCKED;
    }
    if !auto_archive || sender_id <= 0 || chat_id != 0 || !is_from_user {
        return RESULT_PASSED;
    }
    -1
}

pub fn decide(
    sender_id: i64,
    is_service: bool,
    is_contact: bool,
    existing_chat: bool,
    current_user_present: bool,
    current_user_is_bot: bool,
    no_common_group_only: bool,
) -> i32 {
    if !is_service && !is_contact {
        if existing_chat {
            return RESULT_PASSED;
        }
        if !current_user_present {
            increment_pending(sender_id);
            return RESULT_PENDING;
        }
        if current_user_is_bot {
            return RESULT_PASSED;
        }
        return if no_common_group_only {
            RESULT_LOAD_FULL
        } else {
            RESULT_ARCHIVE
        };
    }

    if !current_user_present {
        let attempts = increment_pending(sender_id);
        if attempts > 3 {
            remove_pending(sender_id);
        }
        return RESULT_PENDING;
    }

    RESULT_PASSED
}

fn pending_lock() -> MutexGuard<'static, HashMap<i64, i32>> {
    PENDING_IDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn increment_pending(id: i64) -> i32 {
    let mut pending = pending_lock();
    let value = pending.entry(id).or_insert(0);
    *value += 1;
    *value
}

fn remove_pending(id: i64) {
    pending_lock().remove(&id);
}

pub fn bump_pending(id: i64) {
    increment_pending(id);
}

pub fn pending_is_empty() -> bool {
    pending_lock().is_empty()
}

pub fn take_pending_ids() -> Vec<i64> {
    let mut pending = pending_lock();
    let ids = pending.keys().copied().collect();
    pending.clear();
    ids
}

pub fn validate_regex(pattern: &str, flags: i32) -> bool {
    compile_javaish_regex(pattern, flags).is_ok()
}

pub fn matches(pattern: &str, flags: i32, text: &str, use_opencc: bool) -> Result<bool, String> {
    let regex = compile_javaish_regex(pattern, flags)?;
    if regex.is_match(text).map_err(|e| e.to_string())? {
        return Ok(true);
    }
    if !use_opencc {
        return Ok(false);
    }

    for target in [2, 0] { // TC, SC
        let converted = opencc::convert(target, text)?;
        if regex.is_match(&converted).map_err(|e| e.to_string())? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn compile_javaish_regex(pattern: &str, flags: i32) -> Result<Regex, String> {
    const UNIX_LINES: i32 = 0x01;
    const CASE_INSENSITIVE: i32 = 0x02;
    const COMMENTS: i32 = 0x04;
    const MULTILINE: i32 = 0x08;
    const LITERAL: i32 = 0x10;
    const DOTALL: i32 = 0x20;
    const UNICODE_CASE: i32 = 0x40;
    const CANON_EQ: i32 = 0x80;
    const UNICODE_CHARACTER_CLASS: i32 = 0x100;

    if flags & (UNIX_LINES | CANON_EQ | UNICODE_CHARACTER_CLASS) != 0 {
        return Err("Java Pattern flags require JVM fallback".to_owned());
    }
    if flags & CASE_INSENSITIVE != 0 && flags & UNICODE_CASE == 0 {
        // java.util.regex.Pattern.CASE_INSENSITIVE is ASCII-only unless UNICODE_CASE
        // is also enabled, while Rust regex case folding is Unicode-aware.
        return Err("ASCII-only Java CASE_INSENSITIVE requires JVM fallback".to_owned());
    }

    let mut body = if flags & LITERAL != 0 {
        regex::escape(pattern)
    } else {
        pattern.to_owned()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precheck_matches_expected_shortcuts() {
        assert_eq!(precheck(true, true, true, 5, 0, true), RESULT_BLOCKED);
        assert_eq!(precheck(false, false, false, 5, 0, true), RESULT_PASSED);
        assert_eq!(precheck(false, false, true, 5, 0, true), -1);
    }

    #[test]
    fn pending_state_is_owned_by_rust() {
        assert_eq!(decide(42, false, false, false, false, false, false), RESULT_PENDING);
        assert!(!pending_is_empty());
        assert_eq!(take_pending_ids(), vec![42]);
        assert!(pending_is_empty());
    }

    #[test]
    fn filter_regex_validation_is_owned_by_rust() {
        assert!(validate_regex(r"spam\\d+", 0));
        assert!(!validate_regex("(", 0));
    }

    #[test]
    fn unsupported_java_only_flags_are_rejected_in_rust() {
        assert!(!validate_regex("abc", 0x01));
        assert!(!validate_regex("abc", 0x80));
        assert!(!validate_regex("abc", 0x100));
        assert!(!validate_regex("ä", 0x02));
    }

    #[test]
    fn pending_retry_counter_can_be_requeued_like_the_jvm_implementation() {
        let _ = take_pending_ids();
        bump_pending(77);
        bump_pending(77);
        bump_pending(77);
        assert_eq!(decide(77, true, false, false, false, false, false), RESULT_PENDING);
        assert!(pending_is_empty());
    }
}
