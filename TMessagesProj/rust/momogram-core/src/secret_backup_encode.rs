use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};

pub(crate) fn encode_binary(root: &Value) -> Result<Vec<u8>, String> {
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
        let value = i32::try_from(value).map_err(|_| "Backup collection is too large".to_owned())?;
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
