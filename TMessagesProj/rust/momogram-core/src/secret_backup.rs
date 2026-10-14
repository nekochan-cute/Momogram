use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use pbkdf2::pbkdf2_hmac;
use serde_json::{json, Value};
use sha2::Sha256;
use crate::secret_backup_decode::decode_binary;
use crate::secret_backup_encode::encode_binary;
use crate::secret_backup_io::{read_i32_at, take};

const HEADER: &[u8; 5] = b"NXSCB";
const VERSION: i32 = 4;
const FORMAT_BINARY: i32 = 0;
const FORMAT_JSON: i32 = 1;
const SALT_SIZE: usize = 16;
const IV_SIZE: usize = 12;
const PBKDF2_ITERATIONS_V3: u32 = 10_000;
const PBKDF2_ITERATIONS: u32 = 500_000;

pub fn encode_backup(password: &str, format: i32, state_json: &str) -> Result<Vec<u8>, String> {
    if format != FORMAT_BINARY && format != FORMAT_JSON {
        return Err(format!("Unsupported format {format}"));
    }
    let state: Value = serde_json::from_str(state_json).map_err(|e| format!("Invalid backup state JSON: {e}"))?;
    let payload = if format == FORMAT_BINARY {
        encode_binary(&state)?
    } else {
        serde_json::to_vec(&state).map_err(|e| e.to_string())?
    };

    let mut salt = [0u8; SALT_SIZE];
    let mut iv = [0u8; IV_SIZE];
    getrandom::getrandom(&mut salt).map_err(|e| format!("salt generation failed: {e}"))?;
    getrandom::getrandom(&mut iv).map_err(|e| format!("IV generation failed: {e}"))?;
    let encrypted = encrypt(password, &salt, &iv, &payload, PBKDF2_ITERATIONS)?;

    let mut out = Vec::with_capacity(HEADER.len() + 8 + SALT_SIZE + IV_SIZE + encrypted.len());
    out.extend_from_slice(HEADER);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&format.to_le_bytes());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&iv);
    out.extend_from_slice(&encrypted);
    Ok(out)
}

pub fn decode_backup(password: &str, file: &[u8]) -> Result<String, String> {
    let minimum = HEADER.len() + 8 + SALT_SIZE + IV_SIZE + 16;
    if file.len() < minimum {
        return Err("Backup file is truncated".to_owned());
    }
    if &file[..HEADER.len()] != HEADER {
        return Err("Invalid header".to_owned());
    }
    let mut pos = HEADER.len();
    let version = read_i32_at(file, &mut pos)?;
    let format = read_i32_at(file, &mut pos)?;
    if !(1..=VERSION).contains(&version) {
        return Err(format!("Unsupported version {version}"));
    }
    if format != FORMAT_BINARY && format != FORMAT_JSON {
        return Err(format!("Unsupported format {format}"));
    }
    let salt = take(file, &mut pos, SALT_SIZE)?;
    let iv = take(file, &mut pos, IV_SIZE)?;
    let encrypted = &file[pos..];
    let iterations = if version == 3 {
        PBKDF2_ITERATIONS_V3
    } else {
        PBKDF2_ITERATIONS
    };
    let payload = decrypt(password, salt, iv, encrypted, iterations)?;

    let mut root = if format == FORMAT_BINARY {
        decode_binary(&payload, version)?
    } else {
        serde_json::from_slice::<Value>(&payload).map_err(|e| format!("Invalid JSON payload: {e}"))?
    };
    let object = root
        .as_object_mut()
        .ok_or_else(|| "Backup payload root must be an object".to_owned())?;
    object.insert("_backup_version".to_owned(), json!(version));
    object.insert("_backup_format".to_owned(), json!(format));
    serde_json::to_string(&root).map_err(|e| e.to_string())
}

fn encrypt(password: &str, salt: &[u8], iv: &[u8], payload: &[u8], iterations: u32) -> Result<Vec<u8>, String> {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let result = cipher
        .encrypt(Nonce::from_slice(iv), payload)
        .map_err(|_| "Backup encryption failed".to_owned());
    key.fill(0);
    result
}

fn decrypt(password: &str, salt: &[u8], iv: &[u8], payload: &[u8], iterations: u32) -> Result<Vec<u8>, String> {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let result = cipher
        .decrypt(Nonce::from_slice(iv), payload)
        .map_err(|_| "Backup decryption failed (wrong password or corrupted file)".to_owned());
    key.fill(0);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    fn sample_state() -> Value {
        json!({
            "enc_chats": [{
                "uid": 7, "user": 8, "name": "secret", "data": STANDARD.encode([1,2,3]),
                "ttl": 1, "layer": 2, "seq_in": 3, "seq_out": 4, "use_count": 5,
                "exchange_id": 6, "key_date": 7, "fprint": 8, "in_seq_no": 9,
                "admin_id": 10, "mtproto_seq": 11
            }],
            "messages_by_chat": [{"uid": 7, "messages": [{
                "mid": 1, "date": 2, "data": STANDARD.encode([9]), "random_id": 3,
                "med_data": STANDARD.encode([4,5]), "med_type": 6, "med_date": 7
            }]}],
            "dialogs": [{"did": 30064771072i64, "date": 1, "last_mid": 2}],
            "dialog_settings": [{"did": 30064771072i64, "flags": 123}],
            "users": [{"uid": 8, "name": "u", "status": 1, "data": STANDARD.encode([6])}],
            "params": {"lsv": 1, "sg": 2, "pbytes": STANDARD.encode([7])},
            "requested_holes": [{"uid": 7, "holes": [{"start": 1, "end": 2}]}]
        })
    }

    #[test]
    fn encrypted_binary_backup_roundtrips() {
        let state = sample_state();
        let file = encode_backup("password", FORMAT_BINARY, &state.to_string()).unwrap();
        let decoded: Value = serde_json::from_str(&decode_backup("password", &file).unwrap()).unwrap();
        assert_eq!(decoded["_backup_version"], 4);
        assert_eq!(decoded["enc_chats"][0]["name"], "secret");
        assert_eq!(decoded["messages_by_chat"][0]["messages"][0]["med_type"], 6);
    }

    #[test]
    fn encrypted_json_backup_roundtrips() {
        let state = sample_state();
        let file = encode_backup("password", FORMAT_JSON, &state.to_string()).unwrap();
        let decoded: Value = serde_json::from_str(&decode_backup("password", &file).unwrap()).unwrap();
        assert_eq!(decoded["_backup_format"], FORMAT_JSON);
        assert_eq!(decoded["users"][0]["uid"], 8);
    }

    fn legacy_file(version: i32, format: i32, password: &str, payload: &[u8], iterations: u32) -> Vec<u8> {
        let salt: [u8; SALT_SIZE] = core::array::from_fn(|i| i as u8);
        let iv: [u8; IV_SIZE] = core::array::from_fn(|i| (i + SALT_SIZE) as u8);
        let encrypted = encrypt(password, &salt, &iv, payload, iterations).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(HEADER);
        out.extend_from_slice(&version.to_le_bytes());
        out.extend_from_slice(&format.to_le_bytes());
        out.extend_from_slice(&salt);
        out.extend_from_slice(&iv);
        out.extend_from_slice(&encrypted);
        out
    }

    fn empty_legacy_binary(version: i32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&0i32.to_le_bytes()); // enc_chats
        payload.extend_from_slice(&0i32.to_le_bytes()); // messages_by_chat
        payload.extend_from_slice(&0i32.to_le_bytes()); // dialogs
        if version >= 3 {
            payload.extend_from_slice(&0i32.to_le_bytes()); // dialog_settings
        }
        payload.extend_from_slice(&0i32.to_le_bytes()); // users
        payload.extend_from_slice(&0i32.to_le_bytes()); // params absent
        payload.extend_from_slice(&0i32.to_le_bytes()); // requested_holes
        payload
    }

    #[test]
    fn decodes_legacy_binary_versions_1_and_2() {
        for version in [1, 2] {
            let payload = empty_legacy_binary(version);
            let file = legacy_file(version, FORMAT_BINARY, "password", &payload, PBKDF2_ITERATIONS);
            let decoded: Value = serde_json::from_str(&decode_backup("password", &file).unwrap()).unwrap();
            assert_eq!(decoded["_backup_version"], version);
            assert_eq!(decoded["enc_chats"].as_array().unwrap().len(), 0);
            assert!(decoded["dialog_settings"].as_array().unwrap().is_empty());
        }
    }

    #[test]
    fn decodes_legacy_v3_binary_with_old_iteration_count() {
        let state = sample_state();
        let payload = encode_binary(&state).unwrap();
        let file = legacy_file(3, FORMAT_BINARY, "password", &payload, PBKDF2_ITERATIONS_V3);
        let decoded: Value = serde_json::from_str(&decode_backup("password", &file).unwrap()).unwrap();
        assert_eq!(decoded["_backup_version"], 3);
        assert_eq!(decoded["enc_chats"][0]["uid"], 7);
        assert_eq!(decoded["dialog_settings"][0]["flags"], 123);
    }

    #[test]
    fn java_pbkdf2_unicode_password_vector_matches() {
        let salt: [u8; 16] = core::array::from_fn(|i| i as u8);
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>("päss😀".as_bytes(), &salt, 10_000, &mut key);
        assert_eq!(
            key,
            [
                167, 237, 87, 251, 22, 89, 248, 183,
                245, 4, 111, 196, 211, 60, 199, 17,
                6, 233, 14, 55, 242, 210, 45, 7,
                141, 191, 153, 14, 124, 170, 223, 69,
            ]
        );
    }

    #[test]
    fn rejects_wrong_password_and_truncated_files() {
        let state = sample_state();
        let file = encode_backup("password", FORMAT_JSON, &state.to_string()).unwrap();
        assert!(decode_backup("wrong", &file).is_err());
        assert!(decode_backup("password", &file[..20]).is_err());
    }
}
