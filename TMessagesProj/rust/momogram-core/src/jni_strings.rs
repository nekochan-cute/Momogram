use jni::objects::{JByteArray, JClass, JIntArray, JLongArray, JString};
use jni::sys::{jboolean, jbyteArray, jint, jintArray, jlong, jlongArray, jstring};
use jni::JNIEnv;
use std::ptr;
use crate::{filter, opencc, proxy, secret_backup, strings, url};
use crate::jni_common::*;

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeGetSubString(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
    left: JString,
    right: JString,
    last: jboolean,
) -> jstring {
    let text = match get_string(&mut env, &text) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    let left = match get_optional_string(&mut env, &left) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    let right = match get_optional_string(&mut env, &right) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    match strings::get_sub_string(&text, left.as_deref(), right.as_deref(), last != 0) {
        Ok(value) => new_string(&mut env, value),
        Err(err) => {
            let _ = env.throw_new("java/lang/StringIndexOutOfBoundsException", err);
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeIsInteger(
    mut env: JNIEnv,
    _class: JClass,
    value: JString,
) -> jboolean {
    get_string(&mut env, &value)
        .map(|value| bool_to_jni(strings::is_integer(&value)))
        .unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeReplaceAllRegex(
    mut env: JNIEnv,
    _class: JClass,
    content: JString,
    pattern: JString,
    replacement: JString,
) -> jstring {
    let content = match get_string(&mut env, &content) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    let pattern = match get_string(&mut env, &pattern) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    let replacement = match get_optional_string(&mut env, &replacement) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    match strings::replace_all_regex(&content, &pattern, replacement.as_deref()) {
        Ok(value) => new_string(&mut env, value),
        Err(err) => throw_and_null_string(&mut env, err),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeEscape(
    mut env: JNIEnv,
    _class: JClass,
    content: JString,
) -> jstring {
    match get_string(&mut env, &content) {
        Ok(value) => new_string(&mut env, strings::escape(&value)),
        Err(err) => throw_and_null_string(&mut env, err),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeSimpleUuid(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    new_string(&mut env, strings::simple_uuid())
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeIsRtl(
    mut env: JNIEnv,
    _class: JClass,
    value: JString,
) -> jboolean {
    match get_optional_string(&mut env, &value) {
        Ok(value) => bool_to_jni(strings::is_rtl(value.as_deref())),
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeFirstCharUpper(
    mut env: JNIEnv,
    _class: JClass,
    value: JString,
) -> jstring {
    let value = match get_optional_string(&mut env, &value) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    match strings::first_char_upper(value.as_deref()) {
        Some(value) => new_string(&mut env, value),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativePrefix030(
    mut env: JNIEnv,
    _class: JClass,
    value: JString,
) -> jstring {
    match get_string(&mut env, &value) {
        Ok(value) => new_string(&mut env, strings::prefix_030(&value)),
        Err(err) => throw_and_null_string(&mut env, err),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeSelectName(
    mut env: JNIEnv,
    _class: JClass,
    use_old: jboolean,
    old: JString,
    current: JString,
) -> jstring {
    let old = match get_string(&mut env, &old) { Ok(v) => v, Err(e) => return throw_and_null_string(&mut env, e) };
    let current = match get_string(&mut env, &current) { Ok(v) => v, Err(e) => return throw_and_null_string(&mut env, e) };
    new_string(&mut env, strings::select_name(use_old != 0, &old, &current))
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeIsAppName(
    mut env: JNIEnv,
    _class: JClass,
    value: JString,
    a: JString,
    b: JString,
    c: JString,
) -> jboolean {
    let value = match get_string(&mut env, &value) { Ok(v) => v, Err(_) => return 0 };
    let a = match get_string(&mut env, &a) { Ok(v) => v, Err(_) => return 0 };
    let b = match get_string(&mut env, &b) { Ok(v) => v, Err(_) => return 0 };
    let c = match get_string(&mut env, &c) { Ok(v) => v, Err(_) => return 0 };
    bool_to_jni(strings::is_app_name(&value, &a, &b, &c))
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeAppendCsv(
    mut env: JNIEnv,
    _class: JClass,
    original: JString,
    addition: JString,
) -> jstring {
    let original = match get_optional_string(&mut env, &original) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    let addition = match get_string(&mut env, &addition) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    new_string(&mut env, strings::append_csv(original.as_deref(), &addition))
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeRandomizeFileName(
    mut env: JNIEnv,
    _class: JClass,
    name: JString,
) -> jstring {
    let name = match get_string(&mut env, &name) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    match strings::randomize_file_name(&name) {
        Ok(value) => new_string(&mut env, value),
        Err(err) => throw_and_null_string(&mut env, err),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeMentionRanges(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) -> jintArray {
    let text = match get_string(&mut env, &text) {
        Ok(value) => value,
        Err(err) => {
            let _ = env.throw_new("java/lang/IllegalArgumentException", err);
            return ptr::null_mut();
        }
    };
    let ranges = strings::mention_ranges(&text);
    let array: JIntArray = match env.new_int_array(ranges.len() as i32) {
        Ok(value) => value,
        Err(err) => {
            let _ = env.throw_new("java/lang/RuntimeException", err.to_string());
            return ptr::null_mut();
        }
    };
    if let Err(err) = env.set_int_array_region(&array, 0, &ranges) {
        let _ = env.throw_new("java/lang/RuntimeException", err.to_string());
        return ptr::null_mut();
    }
    array.into_raw()
}

