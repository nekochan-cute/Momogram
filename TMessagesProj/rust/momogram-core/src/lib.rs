mod filter;
mod opencc;
mod proxy;
mod secret_backup;
mod strings;
mod url;

use jni::objects::{JByteArray, JClass, JIntArray, JLongArray, JString};
use jni::sys::{jboolean, jbyteArray, jint, jintArray, jlong, jlongArray, jstring};
use jni::JNIEnv;
use std::ptr;

fn bool_to_jni(value: bool) -> jboolean {
    if value {
        1
    } else {
        0
    }
}

fn get_string(env: &mut JNIEnv, value: &JString) -> Result<String, String> {
    env.get_string(value)
        .map(|s| s.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

fn get_optional_string(env: &mut JNIEnv, value: &JString) -> Result<Option<String>, String> {
    if value.is_null() {
        Ok(None)
    } else {
        get_string(env, value).map(Some)
    }
}

fn new_string(env: &mut JNIEnv, value: String) -> jstring {
    match env.new_string(value) {
        Ok(value) => value.into_raw(),
        Err(err) => {
            let _ = env.throw_new("java/lang/RuntimeException", err.to_string());
            ptr::null_mut()
        }
    }
}

fn throw_and_null_string(env: &mut JNIEnv, err: impl ToString) -> jstring {
    let _ = env.throw_new("java/lang/IllegalArgumentException", err.to_string());
    ptr::null_mut()
}

fn throw_and_null_bytes(env: &mut JNIEnv, err: impl ToString) -> jbyteArray {
    let _ = env.throw_new("java/lang/IllegalArgumentException", err.to_string());
    ptr::null_mut()
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeCleanUrl(
    mut env: JNIEnv,
    _class: JClass,
    src: JString,
    patch_links: jboolean,
    custom_blacklist: JString,
) -> jstring {
    let src = match get_string(&mut env, &src) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    let blacklist = match get_string(&mut env, &custom_blacklist) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    let blacklist: Vec<&str> = blacklist
        .split('\n')
        .filter(|value| !value.is_empty())
        .collect();
    new_string(
        &mut env,
        url::clean_url(&src, patch_links != 0, &blacklist),
    )
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeIsTwitter(
    mut env: JNIEnv,
    _class: JClass,
    host: JString,
) -> jboolean {
    get_string(&mut env, &host)
        .map(|host| bool_to_jni(url::is_twitter(&host)))
        .unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeIsTwitterPreview(
    mut env: JNIEnv,
    _class: JClass,
    host: JString,
) -> jboolean {
    get_string(&mut env, &host)
        .map(|host| bool_to_jni(url::is_twitter_preview(&host)))
        .unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeStripAllQueries(
    mut env: JNIEnv,
    _class: JClass,
    src: JString,
) -> jstring {
    match get_string(&mut env, &src) {
        Ok(src) => new_string(&mut env, url::strip_all_queries(&src)),
        Err(err) => throw_and_null_string(&mut env, err),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeInitOpenCc(
    mut env: JNIEnv,
    _class: JClass,
    bundle: JByteArray,
) -> jboolean {
    let bytes = match env.convert_byte_array(&bundle) {
        Ok(value) => value,
        Err(err) => {
            let _ = env.throw_new("java/lang/IllegalArgumentException", err.to_string());
            return 0;
        }
    };
    match opencc::init(&bytes) {
        Ok(()) => 1,
        Err(err) => {
            let _ = env.throw_new("java/lang/IllegalStateException", err);
            0
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeOpenCcConvert(
    mut env: JNIEnv,
    _class: JClass,
    target: jint,
    input: JString,
) -> jstring {
    let input = match get_string(&mut env, &input) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    match opencc::convert(target, &input) {
        Ok(value) => new_string(&mut env, value),
        Err(err) => throw_and_null_string(&mut env, err),
    }
}

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
    new_string(
        &mut env,
        strings::get_sub_string(&text, left.as_deref(), right.as_deref(), last != 0),
    )
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
    let old = match get_string(&mut env, &old) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    let current = match get_string(&mut env, &current) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    new_string(
        &mut env,
        strings::select_name(use_old != 0, &old, &current),
    )
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
    let value = match get_string(&mut env, &value) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    let a = match get_string(&mut env, &a) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    let b = match get_string(&mut env, &b) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    let c = match get_string(&mut env, &c) {
        Ok(value) => value,
        Err(_) => return 0,
    };
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
    new_string(
        &mut env,
        strings::append_csv(original.as_deref(), &addition),
    )
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

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeParseProxyUri(
    mut env: JNIEnv,
    _class: JClass,
    raw: JString,
) -> jstring {
    let raw = match get_string(&mut env, &raw) {
        Ok(value) => value,
        Err(_) => return ptr::null_mut(),
    };
    let Some(parsed) = proxy::parse_proxy_uri(&raw) else {
        return ptr::null_mut();
    };
    match serde_json::to_string(&parsed) {
        Ok(value) => new_string(&mut env, value),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeProxyNormalizeWebAddress(
    mut env: JNIEnv,
    _class: JClass,
    address: JString,
) -> jstring {
    match get_optional_string(&mut env, &address) {
        Ok(Some(address)) => new_string(&mut env, proxy::normalize_web_address(&address)),
        Ok(None) => new_string(&mut env, String::new()),
        Err(err) => throw_and_null_string(&mut env, err),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeProxyIsValidWebAddress(
    mut env: JNIEnv,
    _class: JClass,
    address: JString,
) -> jboolean {
    match get_optional_string(&mut env, &address) {
        Ok(Some(address)) => bool_to_jni(proxy::is_valid_web_address(&address)),
        _ => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeProxyIsValidWebSecret(
    mut env: JNIEnv,
    _class: JClass,
    secret: JString,
) -> jboolean {
    match get_optional_string(&mut env, &secret) {
        Ok(Some(secret)) => bool_to_jni(proxy::is_valid_web_proxy_secret(&secret)),
        _ => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeProxyIsValidWebPath(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jboolean {
    match get_string(&mut env, &path) {
        Ok(path) => bool_to_jni(proxy::is_valid_web_proxy_path(&path)),
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeProxyDecodeMarkedSecret(
    mut env: JNIEnv,
    _class: JClass,
    secret: JString,
) -> jstring {
    let secret = match get_optional_string(&mut env, &secret) {
        Ok(Some(value)) => value,
        _ => return ptr::null_mut(),
    };
    match proxy::decode_marked_web_proxy_secret(&secret) {
        Some(value) => new_string(&mut env, value),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeProxyEncodeMarkedSecret(
    mut env: JNIEnv,
    _class: JClass,
    secret: JString,
) -> jstring {
    match get_optional_string(&mut env, &secret) {
        Ok(Some(value)) => new_string(&mut env, proxy::encode_marked_web_proxy_secret(&value)),
        Ok(None) => ptr::null_mut(),
        Err(err) => throw_and_null_string(&mut env, err),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeEncodeSecretBackup(
    mut env: JNIEnv,
    _class: JClass,
    password: JString,
    format: jint,
    state_json: JString,
) -> jbyteArray {
    let password = match get_string(&mut env, &password) {
        Ok(value) => value,
        Err(err) => return throw_and_null_bytes(&mut env, err),
    };
    let state_json = match get_string(&mut env, &state_json) {
        Ok(value) => value,
        Err(err) => return throw_and_null_bytes(&mut env, err),
    };
    let bytes = match secret_backup::encode_backup(&password, format, &state_json) {
        Ok(value) => value,
        Err(err) => return throw_and_null_bytes(&mut env, err),
    };
    match env.byte_array_from_slice(&bytes) {
        Ok(value) => value.into_raw(),
        Err(err) => throw_and_null_bytes(&mut env, err),
    }
}

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeDecodeSecretBackup(
    mut env: JNIEnv,
    _class: JClass,
    password: JString,
    file: JByteArray,
) -> jstring {
    let password = match get_string(&mut env, &password) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    let file = match env.convert_byte_array(&file) {
        Ok(value) => value,
        Err(err) => return throw_and_null_string(&mut env, err),
    };
    match secret_backup::decode_backup(&password, &file) {
        Ok(value) => new_string(&mut env, value),
        Err(err) => throw_and_null_string(&mut env, err),
    }
}
