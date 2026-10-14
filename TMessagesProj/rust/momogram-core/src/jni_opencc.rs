use jni::objects::{JClass, JString};
use jni::sys::{jint, jstring};
use jni::JNIEnv;

use crate::jni_common::*;
use crate::opencc;

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
