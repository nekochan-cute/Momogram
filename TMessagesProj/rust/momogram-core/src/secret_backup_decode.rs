use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};

pub(crate) fn decode_binary(payload: &[u8], version: i32) -> Result<Value, String> {
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
        return Err(format!("Binary backup has {} trailing bytes", reader.remaining()));
    }

    let mut root = Map::new();
    root.insert("enc_chats".to_owned(), Value::Array(enc_chats));
    root.insert("messages_by_chat".to_owned(), Value::Array(messages_by_chat));
    root.insert("dialogs".to_owned(), Value::Array(dialogs));
    root.insert("dialog_settings".to_owned(), Value::Array(dialog_settings));
    root.insert("users".to_owned(), Value::Array(users));
    if let Some(params) = params {
        root.insert("params".to_owned(), params);
    }
    root.insert("requested_holes".to_owned(), Value::Array(requested_holes));
    Ok(Value::Object(root))
}

fn put_blob(object: &mut Map<String, Value>, key: &str, data: Option<Vec<u8>>) {
    if let Some(data) = data.filter(|data| !data.is_empty()) {
        object.insert(key.to_owned(), Value::String(STANDARD.encode(data)));
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
                extra[0] as usize | ((extra[1] as usize) << 8) | ((extra[2] as usize) << 16),
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
        String::from_utf8(bytes.to_vec()).map_err(|e| format!("Invalid UTF-8 string in binary backup: {e}"))
    }
}
