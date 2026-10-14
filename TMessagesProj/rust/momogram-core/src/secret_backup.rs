use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use pbkdf2::pbkdf2_hmac;
use serde_json::{json, Map, Value};
use sha2::Sha256;

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
    let state: Value = serde_json::from_str(state_json)
        .map_err(|e| format!("Invalid backup state JSON: {e}"))?;
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
        serde_json::from_slice::<Value>(&payload)
            .map_err(|e| format!("Invalid JSON payload: {e}"))?
    };
    let object = root
        .as_object_mut()
        .ok_or_else(|| "Backup payload root must be an object".to_owned())?;
    object.insert("_backup_version".to_owned(), json!(version));
    object.insert("_backup_format".to_owned(), json!(format));
    serde_json::to_string(&root).map_err(|e| e.to_string())
}

fn encrypt(
    password: &str,
    salt: &[u8],
    iv: &[u8],
    payload: &[u8],
    iterations: u32,
) -> Result<Vec<u8>, String> {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let result = cipher
        .encrypt(Nonce::from_slice(iv), payload)
        .map_err(|_| "Backup encryption failed".to_owned());
    key.fill(0);
    result
}

fn decrypt(
    password: &str,
    salt: &[u8],
    iv: &[u8],
    payload: &[u8],
    iterations: u32,
) -> Result<Vec<u8>, String> {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let result = cipher
        .decrypt(Nonce::from_slice(iv), payload)
        .map_err(|_| "Backup decryption failed (wrong password or corrupted file)".to_owned());
    key.fill(0);
    result
}

fn encode_binary(root: &Value) -> Result<Vec<u8>, String> {
    let mut out = Writer::default();

    let enc_chats = array(root, "enc_chats");
    out.count(enc_chats.len())?;
    for chat in enc_chats {
        let mut row = Writer::default();
        row.i32(i32v(chat, "uid"));
        row.i64(i64v(chat, "user"));
        row.tl_string(strv(chat, "name"))?;
        row.blob(blobv(chat, "data")?.as_deref())?;
        row.blob(blobv(chat, "g")?.as_deref())?;
        row.blob(blobv(chat, "authkey")?.as_deref())?;
        row.i32(i32v(chat, "ttl"));
        row.i32(i32v(chat, "layer"));
        row.i32(i32v(chat, "seq_in"));
        row.i32(i32v(chat, "seq_out"));
        row.i32(i32v(chat, "use_count"));
        row.i64(i64v(chat, "exchange_id"));
        row.i32(i32v(chat, "key_date"));
        row.i64(i64v(chat, "fprint"));
        row.blob(blobv(chat, "fauthkey")?.as_deref())?;
        row.blob(blobv(chat, "khash")?.as_deref())?;
        row.i32(i32v(chat, "in_seq_no"));
        row.i64(i64v(chat, "admin_id"));
        row.i32(i32v(chat, "mtproto_seq"));
        out.count(row.0.len())?;
        out.bytes(&row.0);
    }

    let chats = array(root, "messages_by_chat");
    out.count(chats.len())?;
    for chat in chats {
        out.i32(i32v(chat, "uid"));
        let messages = array(chat, "messages");
        out.count(messages.len())?;
        for message in messages {
            let mut row = Writer::default();
            row.i32(i32v(message, "mid"));
            row.i32(i32v(message, "read_state"));
            row.i32(i32v(message, "send_state"));
            row.i32(i32v(message, "date"));
            row.blob(blobv(message, "data")?.as_deref())?;
            row.i32(i32v(message, "out"));
            row.i32(i32v(message, "ttl"));
            row.i32(i32v(message, "media"));
            row.blob(blobv(message, "replydata")?.as_deref())?;
            row.i32(i32v(message, "imp"));
            row.i32(i32v(message, "mention"));
            row.i32(i32v(message, "forwards"));
            row.blob(blobv(message, "replies_data")?.as_deref())?;
            row.i32(i32v(message, "thread_reply_id"));
            row.i32(i32v(message, "is_channel"));
            row.i32(i32v(message, "reply_to_message_id"));
            row.blob(blobv(message, "custom_params")?.as_deref())?;
            row.i64(i64v(message, "group_id"));
            row.i32(i32v(message, "reply_to_story_id"));
            row.i32(i32v(message, "seq_in"));
            row.i32(i32v(message, "seq_out"));
            row.i64(i64v(message, "random_id"));
            if let Some(media_data) = blobv(message, "med_data")? {
                if !media_data.is_empty() {
                    row.i32(1);
                    row.blob(Some(&media_data))?;
                    row.i32(i32v(message, "med_type"));
                    row.i32(i32v(message, "med_date"));
                } else {
                    row.i32(0);
                }
            } else {
                row.i32(0);
            }
            out.count(row.0.len())?;
            out.bytes(&row.0);
        }
    }

    let dialogs = array(root, "dialogs");
    out.count(dialogs.len())?;
    for dialog in dialogs {
        out.i64(i64v(dialog, "did"));
        out.i32(1);
        out.i32(i32v(dialog, "date"));
        out.i32(i32v(dialog, "unread_count"));
        out.i32(i32v(dialog, "last_mid"));
        out.i32(i32v(dialog, "inbox_max"));
        out.i32(i32v(dialog, "outbox_max"));
        out.i64(i64v(dialog, "last_mid_i"));
        out.i32(i32v(dialog, "unread_count_i"));
        out.i32(i32v(dialog, "pts"));
        out.i32(i32v(dialog, "date_i"));
        out.i32(i32v(dialog, "pinned"));
        out.i32(i32v(dialog, "flags"));
        out.i32(i32v(dialog, "folder_id"));
        out.blob(blobv(dialog, "data")?.as_deref())?;
        out.i32(i32v(dialog, "unread_reactions"));
        out.i64(i64v(dialog, "last_mid_group"));
        out.i32(i32v(dialog, "ttl_period"));
        out.i32(i32v(dialog, "unread_poll_votes"));
    }

    let settings = array(root, "dialog_settings");
    out.count(settings.len())?;
    for setting in settings {
        out.i64(i64v(setting, "did"));
        out.i32(1);
        out.i64(i64v(setting, "flags"));
    }

    let users = array(root, "users");
    out.count(users.len())?;
    for user in users {
        out.i64(i64v(user, "uid"));
        out.i32(1);
        out.blob(blobv(user, "data")?.as_deref())?;
        out.i32(i32v(user, "status"));
        out.tl_string(strv(user, "name"))?;
    }

    if let Some(params) = root.get("params").filter(|value| value.is_object()) {
        out.i32(1);
        out.i32(i32v(params, "lsv"));
        out.i32(i32v(params, "sg"));
        out.blob(blobv(params, "pbytes")?.as_deref())?;
    } else {
        out.i32(0);
    }

    let holes = array(root, "requested_holes");
    out.count(holes.len())?;
    for chat in holes {
        out.i32(i32v(chat, "uid"));
        let rows = array(chat, "holes");
        out.count(rows.len())?;
        for row in rows {
            out.i32(i32v(row, "start"));
            out.i32(i32v(row, "end"));
        }
    }

    Ok(out.0)
}

fn decode_binary(payload: &[u8], version: i32) -> Result<Value, String> {
    let mut reader = Reader::new(payload);
    let mut enc_chats = Vec::new();
    for _ in 0..reader.count()? {
        let row_bytes = reader.length_prefixed_raw()?;
        let mut row = Reader::new(row_bytes);
        let mut object = Map::new();
        object.insert("uid".to_owned(), json!(row.i32()?));
        object.insert("user".to_owned(), json!(row.i64()?));
        object.insert("name".to_owned(), json!(row.tl_string()?));
        put_blob(&mut object, "data", row.blob()?);
        put_blob(&mut object, "g", row.blob()?);
        put_blob(&mut object, "authkey", row.blob()?);
        object.insert("ttl".to_owned(), json!(row.i32()?));
        object.insert("layer".to_owned(), json!(row.i32()?));
        object.insert("seq_in".to_owned(), json!(row.i32()?));
        object.insert("seq_out".to_owned(), json!(row.i32()?));
        object.insert("use_count".to_owned(), json!(row.i32()?));
        object.insert("exchange_id".to_owned(), json!(row.i64()?));
        object.insert("key_date".to_owned(), json!(row.i32()?));
        object.insert("fprint".to_owned(), json!(row.i64()?));
        put_blob(&mut object, "fauthkey", row.blob()?);
        put_blob(&mut object, "khash", row.blob()?);
        object.insert("in_seq_no".to_owned(), json!(row.i32()?));
        object.insert("admin_id".to_owned(), json!(row.i64()?));
        object.insert("mtproto_seq".to_owned(), json!(row.i32()?));
        enc_chats.push(Value::Object(object));
    }

    let mut messages_by_chat = Vec::new();
    for _ in 0..reader.count()? {
        let uid = reader.i32()?;
        let mut messages = Vec::new();
        for _ in 0..reader.count()? {
            let row_bytes = reader.length_prefixed_raw()?;
            let mut row = Reader::new(row_bytes);
            let mut object = Map::new();
            object.insert("mid".to_owned(), json!(row.i32()?));
            object.insert("read_state".to_owned(), json!(row.i32()?));
            object.insert("send_state".to_owned(), json!(row.i32()?));
            object.insert("date".to_owned(), json!(row.i32()?));
            put_blob(&mut object, "data", row.blob()?);
            object.insert("out".to_owned(), json!(row.i32()?));
            object.insert("ttl".to_owned(), json!(row.i32()?));
            object.insert("media".to_owned(), json!(row.i32()?));
            put_blob(&mut object, "replydata", row.blob()?);
            object.insert("imp".to_owned(), json!(row.i32()?));
            object.insert("mention".to_owned(), json!(row.i32()?));
            object.insert("forwards".to_owned(), json!(row.i32()?));
            put_blob(&mut object, "replies_data", row.blob()?);
            object.insert("thread_reply_id".to_owned(), json!(row.i32()?));
            object.insert("is_channel".to_owned(), json!(row.i32()?));
            object.insert("reply_to_message_id".to_owned(), json!(row.i32()?));
            put_blob(&mut object, "custom_params", row.blob()?);
            object.insert("group_id".to_owned(), json!(row.i64()?));
            object.insert("reply_to_story_id".to_owned(), json!(row.i32()?));
            object.insert("seq_in".to_owned(), json!(row.i32()?));
            object.insert("seq_out".to_owned(), json!(row.i32()?));
            object.insert("random_id".to_owned(), json!(row.i64()?));
            if version >= 2 && row.i32()? == 1 {
                put_blob(&mut object, "med_data", row.blob()?);
                object.insert("med_type".to_owned(), json!(row.i32()?));
                object.insert("med_date".to_owned(), json!(row.i32()?));
            }
            messages.push(Value::Object(object));
        }
        messages_by_chat.push(json!({"uid": uid, "messages": messages}));
    }

    let mut dialogs = Vec::new();
    for _ in 0..reader.count()? {
        let did = reader.i64()?;
        if reader.i32()? == 0 {
            continue;
        }
        let mut object = Map::new();
        object.insert("did".to_owned(), json!(did));
        object.insert("date".to_owned(), json!(reader.i32()?));
        object.insert("unread_count".to_owned(), json!(reader.i32()?));
        object.insert("last_mid".to_owned(), json!(reader.i32()?));
        object.insert("inbox_max".to_owned(), json!(reader.i32()?));
        object.insert("outbox_max".to_owned(), json!(reader.i32()?));
        object.insert("last_mid_i".to_owned(), json!(reader.i64()?));
        object.insert("unread_count_i".to_owned(), json!(reader.i32()?));
        object.insert("pts".to_owned(), json!(reader.i32()?));
        object.insert("date_i".to_owned(), json!(reader.i32()?));
        object.insert("pinned".to_owned(), json!(reader.i32()?));
        object.insert("flags".to_owned(), json!(reader.i32()?));
        object.insert("folder_id".to_owned(), json!(reader.i32()?));
        put_blob(&mut object, "data", reader.blob()?);
        object.insert("unread_reactions".to_owned(), json!(reader.i32()?));
        object.insert("last_mid_group".to_owned(), json!(reader.i64()?));
        object.insert("ttl_period".to_owned(), json!(reader.i32()?));
        object.insert("unread_poll_votes".to_owned(), json!(reader.i32()?));
        dialogs.push(Value::Object(object));
    }

    let mut dialog_settings = Vec::new();
    if version >= 3 {
        for _ in 0..reader.count()? {
            let did = reader.i64()?;
            if reader.i32()? == 0 {
                continue;
            }
            dialog_settings.push(json!({"did": did, "flags": reader.i64()?}));
        }
    }

    let mut users = Vec::new();
    for _ in 0..reader.count()? {
        let uid = reader.i64()?;
        if reader.i32()? == 0 {
            continue;
        }
        let mut object = Map::new();
        object.insert("uid".to_owned(), json!(uid));
        put_blob(&mut object, "data", reader.blob()?);
        object.insert("status".to_owned(), json!(reader.i32()?));
        object.insert("name".to_owned(), json!(reader.tl_string()?));
        users.push(Value::Object(object));
    }

    let params = if reader.i32()? == 1 {
        let mut object = Map::new();
        object.insert("lsv".to_owned(), json!(reader.i32()?));
        object.insert("sg".to_owned(), json!(reader.i32()?));
        put_blob(&mut object, "pbytes", reader.blob()?);
        Some(Value::Object(object))
    } else {
        None
    };

    let mut requested_holes = Vec::new();
    for _ in 0..reader.count()? {
        let uid = reader.i32()?;
        let mut holes = Vec::new();
        for _ in 0..reader.count()? {
            holes.push(json!({"start": reader.i32()?, "end": reader.i32()?}));
        }
        requested_holes.push(json!({"uid": uid, "holes": holes}));
    }

    if reader.remaining() != 0 {
        return Err(format!(
            "Binary backup has {} trailing bytes",
            reader.remaining()
        ));
    }

    let mut root = Map::new();
    root.insert("enc_chats".to_owned(), Value::Array(enc_chats));
    root.insert(
        "messages_by_chat".to_owned(),
        Value::Array(messages_by_chat),
    );
    root.insert("dialogs".to_owned(), Value::Array(dialogs));
    root.insert(
        "dialog_settings".to_owned(),
        Value::Array(dialog_settings),
    );
    root.insert("users".to_owned(), Value::Array(users));
    if let Some(params) = params {
        root.insert("params".to_owned(), params);
    }
    root.insert(
        "requested_holes".to_owned(),
        Value::Array(requested_holes),
    );
    Ok(Value::Object(root))
}

fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn i32v(value: &Value, key: &str) -> i32 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0) as i32
}

fn i64v(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn strv<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn blobv(value: &Value, key: &str) -> Result<Option<Vec<u8>>, String> {
    let Some(encoded) = value.get(key).and_then(Value::as_str) else {
        return Ok(None);
    };
    if encoded.is_empty() {
        return Ok(None);
    }
    STANDARD
        .decode(encoded)
        .map(Some)
        .map_err(|e| format!("Invalid base64 field {key}: {e}"))
}

fn put_blob(object: &mut Map<String, Value>, key: &str, data: Option<Vec<u8>>) {
    if let Some(data) = data.filter(|data| !data.is_empty()) {
        object.insert(key.to_owned(), Value::String(STANDARD.encode(data)));
    }
}

#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn i32(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        self.0.extend_from_slice(value);
    }

    fn count(&mut self, value: usize) -> Result<(), String> {
        let value =
            i32::try_from(value).map_err(|_| "Backup collection is too large".to_owned())?;
        self.i32(value);
        Ok(())
    }

    fn blob(&mut self, value: Option<&[u8]>) -> Result<(), String> {
        let value = value.unwrap_or(&[]);
        self.count(value.len())?;
        self.bytes(value);
        Ok(())
    }

    fn tl_string(&mut self, value: &str) -> Result<(), String> {
        let data = value.as_bytes();
        if data.len() > 0x00ff_ffff {
            return Err("String is too large for Telegram serialization".to_owned());
        }
        let header_len;
        if data.len() <= 253 {
            self.0.push(data.len() as u8);
            header_len = 1;
        } else {
            self.0.push(254);
            self.0.push((data.len() & 0xff) as u8);
            self.0.push(((data.len() >> 8) & 0xff) as u8);
            self.0.push(((data.len() >> 16) & 0xff) as u8);
            header_len = 4;
        }
        self.bytes(data);
        let mut total = data.len() + header_len;
        while total % 4 != 0 {
            self.0.push(0);
            total += 1;
        }
        Ok(())
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| "Backup length overflow".to_owned())?;
        if end > self.data.len() {
            return Err("Unexpected end of binary backup".to_owned());
        }
        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.exact(4)?.try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.exact(8)?.try_into().unwrap()))
    }

    fn count(&mut self) -> Result<usize, String> {
        let value = self.i32()?;
        if value < 0 {
            return Err("Negative collection length in binary backup".to_owned());
        }
        Ok(value as usize)
    }

    fn length_prefixed_raw(&mut self) -> Result<&'a [u8], String> {
        let len = self.count()?;
        self.exact(len)
    }

    fn blob(&mut self) -> Result<Option<Vec<u8>>, String> {
        let len = self.i32()?;
        if len <= 0 {
            return Ok(None);
        }
        Ok(Some(self.exact(len as usize)?.to_vec()))
    }

    fn tl_string(&mut self) -> Result<String, String> {
        let first = *self.exact(1)?.first().unwrap() as usize;
        let (len, header_len) = if first >= 254 {
            let extra = self.exact(3)?;
            (
                extra[0] as usize
                    | ((extra[1] as usize) << 8)
                    | ((extra[2] as usize) << 16),
                4usize,
            )
        } else {
            (first, 1usize)
        };
        let bytes = self.exact(len)?;
        let mut total = len + header_len;
        while total % 4 != 0 {
            self.exact(1)?;
            total += 1;
        }
        String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("Invalid UTF-8 string in binary backup: {e}"))
    }
}

fn read_i32_at(data: &[u8], pos: &mut usize) -> Result<i32, String> {
    Ok(i32::from_le_bytes(
        take(data, pos, 4)?.try_into().unwrap(),
    ))
}

fn take<'a>(data: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8], String> {
    let end = (*pos)
        .checked_add(len)
        .ok_or_else(|| "Backup length overflow".to_owned())?;
    if end > data.len() {
        return Err("Unexpected end of backup file".to_owned());
    }
    let out = &data[*pos..end];
    *pos = end;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn binary_codec_roundtrips() {
        let state = sample_state();
        let bytes = encode_binary(&state).unwrap();
        let decoded = decode_binary(&bytes, 4).unwrap();
        assert_eq!(decoded["enc_chats"][0]["uid"], 7);
        assert_eq!(
            decoded["messages_by_chat"][0]["messages"][0]["med_type"],
            6
        );
        assert_eq!(decoded["dialog_settings"][0]["flags"], 123);
    }

    #[test]
    fn encrypted_binary_backup_roundtrips() {
        let state = sample_state();
        let file = encode_backup("password", FORMAT_BINARY, &state.to_string()).unwrap();
        let decoded: Value =
            serde_json::from_str(&decode_backup("password", &file).unwrap()).unwrap();
        assert_eq!(decoded["_backup_version"], 4);
        assert_eq!(decoded["enc_chats"][0]["name"], "secret");
    }

    #[test]
    fn encrypted_json_backup_roundtrips() {
        let state = sample_state();
        let file = encode_backup("password", FORMAT_JSON, &state.to_string()).unwrap();
        let decoded: Value =
            serde_json::from_str(&decode_backup("password", &file).unwrap()).unwrap();
        assert_eq!(decoded["_backup_format"], FORMAT_JSON);
        assert_eq!(decoded["users"][0]["uid"], 8);
    }
}
