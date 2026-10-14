mod url;

use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jstring};
use jni::JNIEnv;
use std::ptr;

#[no_mangle]
pub extern "system" fn Java_moe_hx030_momogram_utils_RustCore_nativeCleanUrl(
    mut env: JNIEnv,
    _class: JClass,
    src: JString,
    patch_links: jboolean,
    custom_blacklist: JString,
) -> jstring {
    let src = match env.get_string(&src) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(_) => return ptr::null_mut(),
    };
    let blacklist = match env.get_string(&custom_blacklist) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(_) => return ptr::null_mut(),
    };
    let blacklist: Vec<&str> = blacklist.split('\n').filter(|value| !value.is_empty()).collect();
    let cleaned = url::clean_url(&src, patch_links != 0, &blacklist);

    match env.new_string(cleaned) {
        Ok(value) => value.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}
