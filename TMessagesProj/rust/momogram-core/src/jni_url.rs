use jni::objects::{JByteArray, JClass, JIntArray, JLongArray, JString};
use jni::sys::{jboolean, jbyteArray, jint, jintArray, jlong, jlongArray, jstring};
use jni::JNIEnv;
use std::ptr;
use crate::{filter, opencc, proxy, secret_backup, strings, url};
use crate::jni_common::*;

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
    let blacklist: Vec<&str> = blacklist.split('\n').filter(|value| !value.is_empty()).collect();
    new_string(&mut env, url::clean_url(&src, patch_links != 0, &blacklist))
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
        Ok(src) => match url::strip_all_queries(&src) {
            Ok(value) => new_string(&mut env, value),
            Err(err) => {
                let _ = env.throw_new("java/lang/StringIndexOutOfBoundsException", err);
                ptr::null_mut()
            }
        },
        Err(err) => throw_and_null_string(&mut env, err),
    }
}

