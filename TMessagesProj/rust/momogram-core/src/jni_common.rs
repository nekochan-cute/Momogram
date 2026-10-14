use jni::objects::JString;
use jni::sys::{jboolean,jbyteArray,jstring};
use jni::JNIEnv;
use std::ptr;

pub(crate) fn bool_to_jni(value: bool) -> jboolean {
    if value { 1 } else { 0 }
}

pub(crate) fn get_string(env: &mut JNIEnv, value: &JString) -> Result<String, String> {
    env.get_string(value)
        .map(|s| s.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

pub(crate) fn get_optional_string(env: &mut JNIEnv, value: &JString) -> Result<Option<String>, String> {
    if value.is_null() {
        Ok(None)
    } else {
        get_string(env, value).map(Some)
    }
}

pub(crate) fn new_string(env: &mut JNIEnv, value: String) -> jstring {
    match env.new_string(value) {
        Ok(value) => value.into_raw(),
        Err(err) => {
            let _ = env.throw_new("java/lang/RuntimeException", err.to_string());
            ptr::null_mut()
        }
    }
}

pub(crate) fn throw_and_null_string(env: &mut JNIEnv, err: impl ToString) -> jstring {
    let _ = env.throw_new("java/lang/IllegalArgumentException", err.to_string());
    ptr::null_mut()
}

pub(crate) fn throw_and_null_bytes(env: &mut JNIEnv, err: impl ToString) -> jbyteArray {
    let _ = env.throw_new("java/lang/IllegalArgumentException", err.to_string());
    ptr::null_mut()
}

