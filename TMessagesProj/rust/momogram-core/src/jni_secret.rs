use jni::objects::{JByteArray, JClass, JIntArray, JLongArray, JString};
use jni::sys::{jboolean, jbyteArray, jint, jintArray, jlong, jlongArray, jstring};
use jni::JNIEnv;
use std::ptr;
use crate::{filter, opencc, proxy, secret_backup, strings, url};
use crate::jni_common::*;

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
