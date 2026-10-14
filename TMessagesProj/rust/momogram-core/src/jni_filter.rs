use jni::objects::{JByteArray, JClass, JIntArray, JLongArray, JString};
use jni::sys::{jboolean, jbyteArray, jint, jintArray, jlong, jlongArray, jstring};
use jni::JNIEnv;
use std::ptr;
use crate::{filter, opencc, proxy, secret_backup, strings, url};
use crate::jni_common::*;

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeFilterPrecheck(
    _env: JNIEnv,
    _class: JClass,
    ignore_blocked: jboolean,
    is_blocked: jboolean,
    auto_archive: jboolean,
    sender_id: jlong,
    chat_id: jlong,
    is_from_user: jboolean,
) -> jint {
    filter::precheck(
        ignore_blocked != 0,
        is_blocked != 0,
        auto_archive != 0,
        sender_id,
        chat_id,
        is_from_user != 0,
    )
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeFilterDecide(
    _env: JNIEnv,
    _class: JClass,
    sender_id: jlong,
    is_service: jboolean,
    is_contact: jboolean,
    existing_chat: jboolean,
    current_user_present: jboolean,
    current_user_is_bot: jboolean,
    no_common_group_only: jboolean,
) -> jint {
    filter::decide(
        sender_id,
        is_service != 0,
        is_contact != 0,
        existing_chat != 0,
        current_user_present != 0,
        current_user_is_bot != 0,
        no_common_group_only != 0,
    )
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeBumpPendingId(
    _env: JNIEnv,
    _class: JClass,
    sender_id: jlong,
) {
    filter::bump_pending(sender_id);
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativePendingIsEmpty(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    bool_to_jni(filter::pending_is_empty())
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeTakePendingIds(
    mut env: JNIEnv,
    _class: JClass,
) -> jlongArray {
    let ids = filter::take_pending_ids();
    let array: JLongArray = match env.new_long_array(ids.len() as i32) {
        Ok(value) => value,
        Err(err) => {
            let _ = env.throw_new("java/lang/RuntimeException", err.to_string());
            return ptr::null_mut();
        }
    };
    if let Err(err) = env.set_long_array_region(&array, 0, &ids) {
        let _ = env.throw_new("java/lang/RuntimeException", err.to_string());
        return ptr::null_mut();
    }
    array.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeFilterRegexValid(
    mut env: JNIEnv,
    _class: JClass,
    pattern: JString,
) -> jboolean {
    match get_string(&mut env, &pattern) {
        Ok(pattern) => bool_to_jni(filter::validate_regex(&pattern, 0)),
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeFilterMatches(
    mut env: JNIEnv,
    _class: JClass,
    pattern: JString,
    flags: jint,
    text: JString,
    use_opencc: jboolean,
) -> jint {
    let pattern = match get_string(&mut env, &pattern) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    let text = match get_string(&mut env, &text) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    match filter::matches(&pattern, flags, &text, use_opencc != 0) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

