use jni::objects::{JByteArray, JClass, JIntArray, JLongArray, JString};
use jni::sys::{jboolean, jbyteArray, jint, jintArray, jlong, jlongArray, jstring};
use jni::JNIEnv;
use std::ptr;
use crate::{filter, opencc, proxy, secret_backup, strings, url};
use crate::jni_common::*;

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

