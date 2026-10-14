use jni::objects::{JByteArray, JClass, JIntArray, JLongArray, JString};
use jni::sys::{jboolean, jbyteArray, jint, jintArray, jlong, jlongArray, jstring};
use jni::JNIEnv;
use std::ptr;
use crate::{filter, opencc, proxy, secret_backup, strings, url};
use crate::jni_common::*;

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

