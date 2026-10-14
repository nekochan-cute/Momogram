package moe.hx030.momogram.helpers;

import android.util.Base64;
import android.util.Log;

import androidx.annotation.Nullable;

import org.json.JSONArray;
import org.json.JSONObject;
import org.telegram.SQLite.SQLiteCursor;
import org.telegram.SQLite.SQLiteDatabase;
import org.telegram.SQLite.SQLitePreparedStatement;
import org.telegram.messenger.AndroidUtilities;
import org.telegram.messenger.BuildConfig;
import org.telegram.messenger.DialogObject;
import org.telegram.messenger.FileLog;
import org.telegram.messenger.MessagesController;
import org.telegram.messenger.MessagesStorage;
import org.telegram.messenger.NotificationCenter;
import org.telegram.tgnet.NativeByteBuffer;
import org.telegram.tgnet.TLRPC;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.util.ArrayList;
import java.util.Locale;

import moe.hx030.momogram.utils.RustCore;

public class SecretChatBackupManager {

    private static final String TAG = "SecretChatBackupManager";
    public static final int FORMAT_BINARY = 0;
    public static final int FORMAT_JSON = 1;

    public interface BackupDelegate {
        void onProgress(float progress);
        void onFinish(boolean success, String error, String logs);
    }

    private static final class RestoreAccumulator {
        final ArrayList<Integer> restoredChatIds = new ArrayList<>();
        final ArrayList<Long> restoredDialogIds = new ArrayList<>();
        final ArrayList<Integer> createdChatIds = new ArrayList<>();
    }

    private static void log(StringBuilder logs, String message) {
        if (logs != null) {
            logs.append(message).append('\n');
        }
        Log.d(TAG, message);
    }

    public static void backup(int currentAccount, String password, File outputFile, int format, BackupDelegate delegate) {
        final String finalPassword = BuildConfig.OFFICIAL_VERSION + BuildConfig.SALT_030 + password;
        StringBuilder logs = new StringBuilder();
        MessagesStorage storage = MessagesStorage.getInstance(currentAccount);
        storage.getStorageQueue().postRunnable(() -> {
            try {
                log(logs, "Starting backup...");
                SQLiteDatabase database = storage.getDatabase();
                if (database == null) {
                    finish(delegate, false, "Database is null", logs);
                    return;
                }

                ArrayList<Integer> chatIds = new ArrayList<>();
                String idsToLoad = loadEncryptedChatIds(database, chatIds);
                if (chatIds.isEmpty()) {
                    finish(delegate, false, "No secret chats found", logs);
                    return;
                }
                log(logs, "Found " + chatIds.size() + " chats.");
                log(logs, "Extracting database state...");
                JSONObject state = extractDatabaseState(database, idsToLoad, chatIds, logs);

                log(logs, format == FORMAT_BINARY ? "Rust: serializing Binary backup..." : "Rust: serializing JSON backup...");
                byte[] encoded = RustCore.encodeSecretBackup(finalPassword, format, state.toString());
                try (FileOutputStream fos = new FileOutputStream(outputFile)) {
                    fos.write(encoded);
                }

                log(logs, "Backup written to " + outputFile.getAbsolutePath());
                finish(delegate, true, null, logs);
            } catch (Exception e) {
                FileLog.e(e);
                finish(delegate, false, e.getMessage() != null ? e.getMessage() : e.getClass().getName(), logs);
            }
        });
    }

    public static void restore(int currentAccount, String password, File inputFile, BackupDelegate delegate) {
        final String finalPassword = BuildConfig.OFFICIAL_VERSION + BuildConfig.SALT_030 + password;
        StringBuilder logs = new StringBuilder();
        MessagesStorage storage = MessagesStorage.getInstance(currentAccount);
        storage.getStorageQueue().postRunnable(() -> {
            try {
                SQLiteDatabase database = storage.getDatabase();
                if (database == null) {
                    finish(delegate, false, "Database is null", logs);
                    return;
                }

                log(logs, "Rust: decrypting and decoding backup...");
                byte[] fileBytes = new byte[(int) inputFile.length()];
                try (FileInputStream fis = new FileInputStream(inputFile)) {
                    readFully(fis, fileBytes);
                }
                String decoded = RustCore.decodeSecretBackup(finalPassword, fileBytes);
                JSONObject root = new JSONObject(decoded);
                int version = root.optInt("_backup_version", 4);
                root.remove("_backup_version");
                root.remove("_backup_format");
                log(logs, "Decryption successful.");

                RestoreAccumulator accumulator = new RestoreAccumulator();
                database.beginTransaction();
                try {
                    applyDatabaseState(database, root, version, accumulator, logs);
                } finally {
                    database.commitTransaction();
                }

                rehydrateRestoredDialogs(currentAccount, storage, accumulator, logs);
                finish(delegate, true, null, logs);
            } catch (Exception e) {
                FileLog.e(e);
                finish(delegate, false, e.getMessage() != null ? e.getMessage() : e.getClass().getName(), logs);
            }
        });
    }

    private static JSONObject extractDatabaseState(SQLiteDatabase database, String idsToLoad, ArrayList<Integer> chatIds, StringBuilder logs) throws Exception {
        JSONObject root = new JSONObject();
        ArrayList<Long> userIds = new ArrayList<>();

        log(logs, "Extracting enc_chats...");
        root.put("enc_chats", buildEncryptedChatsJson(database, idsToLoad, userIds));
        log(logs, "Extracting messages...");
        root.put("messages_by_chat", buildMessagesJson(database, chatIds));
        log(logs, "Extracting dialogs...");
        root.put("dialogs", buildDialogsJson(database, chatIds));
        log(logs, "Extracting dialog_settings...");
        root.put("dialog_settings", buildDialogSettingsJson(database, chatIds));
        log(logs, "Extracting users...");
        root.put("users", buildUsersJson(database, userIds));
        log(logs, "Extracting params...");
        JSONObject params = buildParamsJson(database);
        if (params != null) root.put("params", params);
        log(logs, "Extracting requested_holes...");
        root.put("requested_holes", buildRequestedHolesJson(database, chatIds));
        return root;
    }

    private static void applyDatabaseState(SQLiteDatabase database, JSONObject root, int version, RestoreAccumulator accumulator, StringBuilder logs) throws Exception {
        log(logs, "Restoring enc_chats...");
        readEncryptedChatsJson(database, root.optJSONArray("enc_chats"), accumulator);
        log(logs, "Restoring messages...");
        readMessagesJson(database, root.optJSONArray("messages_by_chat"), accumulator);
        log(logs, "Restoring dialogs...");
        readDialogsJson(database, root.optJSONArray("dialogs"), accumulator);
        if (version >= 3) {
            log(logs, "Restoring dialog_settings...");
            readDialogSettingsJson(database, root.optJSONArray("dialog_settings"));
        } else {
            log(logs, "Backup version does not contain dialog_settings; mute state cannot be restored from this file.");
        }
        log(logs, "Restoring users...");
        readUsersJson(database, root.optJSONArray("users"));
        log(logs, "Restoring params...");
        readParamsJson(database, root.optJSONObject("params"));
        log(logs, "Restoring requested_holes...");
        readRequestedHolesJson(database, root.optJSONArray("requested_holes"));
    }

    private static String loadEncryptedChatIds(SQLiteDatabase database, ArrayList<Integer> chatIds) throws Exception {
        SQLiteCursor cursor = database.queryFinalized("SELECT uid FROM enc_chats");
        try {
            StringBuilder ids = new StringBuilder();
            while (cursor.next()) {
                int uid = cursor.intValue(0);
                chatIds.add(uid);
                if (ids.length() > 0) {
                    ids.append(',');
                }
                ids.append(uid);
            }
            return ids.toString();
        } finally {
            cursor.dispose();
        }
    }

    private static JSONArray buildEncryptedChatsJson(SQLiteDatabase database, String idsToLoad, ArrayList<Long> userIds) throws Exception {
        JSONArray array = new JSONArray();
        SQLiteCursor cursor = database.queryFinalized(String.format(Locale.US,
                "SELECT uid, user, name, data, g, authkey, ttl, layer, seq_in, seq_out, use_count, exchange_id, key_date, fprint, fauthkey, khash, in_seq_no, admin_id, mtproto_seq FROM enc_chats WHERE uid IN(%s)",
                idsToLoad));
        try {
            while (cursor.next()) {
                JSONObject chat = new JSONObject();
                long userId = cursor.longValue(1);
                addUnique(userIds, userId);
                chat.put("uid", cursor.intValue(0));
                chat.put("user", userId);
                chat.put("name", cursor.stringValue(2));
                putBase64(chat, "data", getBufferBytes(cursor.byteBufferValue(3)));
                putBase64(chat, "g", cursor.byteArrayValue(4));
                putBase64(chat, "authkey", cursor.byteArrayValue(5));
                chat.put("ttl", cursor.intValue(6));
                chat.put("layer", cursor.intValue(7));
                chat.put("seq_in", cursor.intValue(8));
                chat.put("seq_out", cursor.intValue(9));
                chat.put("use_count", cursor.intValue(10));
                chat.put("exchange_id", cursor.longValue(11));
                chat.put("key_date", cursor.intValue(12));
                chat.put("fprint", cursor.longValue(13));
                putBase64(chat, "fauthkey", cursor.byteArrayValue(14));
                putBase64(chat, "khash", cursor.byteArrayValue(15));
                chat.put("in_seq_no", cursor.intValue(16));
                chat.put("admin_id", cursor.longValue(17));
                chat.put("mtproto_seq", cursor.intValue(18));
                array.put(chat);
            }
        } finally {
            cursor.dispose();
        }
        return array;
    }

    private static JSONArray buildMessagesJson(SQLiteDatabase database, ArrayList<Integer> chatIds) throws Exception {
        JSONArray chatsArray = new JSONArray();
        for (int chatId : chatIds) {
            long dialogId = DialogObject.makeEncryptedDialogId(chatId);
            JSONObject chat = new JSONObject();
            chat.put("uid", chatId);
            JSONArray messages = new JSONArray();

            SQLiteCursor cursor = database.queryFinalized("SELECT m.mid, m.read_state, m.send_state, m.date, m.data, m.out, m.ttl, m.media, m.replydata, m.imp, m.mention, m.forwards, m.replies_data, m.thread_reply_id, m.is_channel, m.reply_to_message_id, m.custom_params, m.group_id, m.reply_to_story_id, s.seq_in, s.seq_out, r.random_id, med.data, med.type, med.date FROM messages_v2 AS m " +
                    "LEFT JOIN messages_seq AS s ON m.mid = s.mid " +
                    "LEFT JOIN randoms_v2 AS r ON m.mid = r.mid AND m.uid = r.uid " +
                    "LEFT JOIN media_v4 AS med ON m.mid = med.mid AND m.uid = med.uid " +
                    "WHERE m.uid = ?", dialogId);
            try {
                while (cursor.next()) {
                    JSONObject message = new JSONObject();
                    message.put("mid", cursor.intValue(0));
                    message.put("read_state", cursor.intValue(1));
                    message.put("send_state", cursor.intValue(2));
                    message.put("date", cursor.intValue(3));
                    putBase64(message, "data", getBufferBytes(cursor.byteBufferValue(4)));
                    message.put("out", cursor.intValue(5));
                    message.put("ttl", cursor.intValue(6));
                    message.put("media", cursor.intValue(7));
                    putBase64(message, "replydata", cursor.byteArrayValue(8));
                    message.put("imp", cursor.intValue(9));
                    message.put("mention", cursor.intValue(10));
                    message.put("forwards", cursor.intValue(11));
                    putBase64(message, "replies_data", cursor.byteArrayValue(12));
                    message.put("thread_reply_id", cursor.intValue(13));
                    message.put("is_channel", cursor.intValue(14));
                    message.put("reply_to_message_id", cursor.intValue(15));
                    putBase64(message, "custom_params", cursor.byteArrayValue(16));
                    message.put("group_id", cursor.longValue(17));
                    message.put("reply_to_story_id", cursor.intValue(18));
                    message.put("seq_in", cursor.intValue(19));
                    message.put("seq_out", cursor.intValue(20));
                    message.put("random_id", cursor.longValue(21));
                    putBase64(message, "med_data", getBufferBytes(cursor.byteBufferValue(22)));
                    if (!message.isNull("med_data")) {
                        message.put("med_type", cursor.intValue(23));
                        message.put("med_date", cursor.intValue(24));
                    }
                    messages.put(message);
                }
            } finally {
                cursor.dispose();
            }

            chat.put("messages", messages);
            chatsArray.put(chat);
        }
        return chatsArray;
    }

    private static JSONArray buildDialogsJson(SQLiteDatabase database, ArrayList<Integer> chatIds) throws Exception {
        JSONArray dialogs = new JSONArray();
        for (int chatId : chatIds) {
            long dialogId = DialogObject.makeEncryptedDialogId(chatId);
            SQLiteCursor cursor = database.queryFinalized("SELECT date, unread_count, last_mid, inbox_max, outbox_max, last_mid_i, unread_count_i, pts, date_i, pinned, flags, folder_id, data, unread_reactions, last_mid_group, ttl_period, unread_poll_votes FROM dialogs WHERE did = ?", dialogId);
            try {
                if (cursor.next()) {
                    JSONObject dialog = new JSONObject();
                    dialog.put("did", dialogId);
                    dialog.put("date", cursor.intValue(0));
                    dialog.put("unread_count", cursor.intValue(1));
                    dialog.put("last_mid", cursor.intValue(2));
                    dialog.put("inbox_max", cursor.intValue(3));
                    dialog.put("outbox_max", cursor.intValue(4));
                    dialog.put("last_mid_i", cursor.longValue(5));
                    dialog.put("unread_count_i", cursor.intValue(6));
                    dialog.put("pts", cursor.intValue(7));
                    dialog.put("date_i", cursor.intValue(8));
                    dialog.put("pinned", cursor.intValue(9));
                    dialog.put("flags", cursor.intValue(10));
                    dialog.put("folder_id", cursor.intValue(11));
                    putBase64(dialog, "data", getBufferBytes(cursor.byteBufferValue(12)));
                    dialog.put("unread_reactions", cursor.intValue(13));
                    dialog.put("last_mid_group", cursor.longValue(14));
                    dialog.put("ttl_period", cursor.intValue(15));
                    dialog.put("unread_poll_votes", cursor.intValue(16));
                    dialogs.put(dialog);
                }
            } finally {
                cursor.dispose();
            }
        }
        return dialogs;
    }

    private static JSONArray buildDialogSettingsJson(SQLiteDatabase database, ArrayList<Integer> chatIds) throws Exception {
        JSONArray settings = new JSONArray();
        for (int chatId : chatIds) {
            long dialogId = DialogObject.makeEncryptedDialogId(chatId);
            SQLiteCursor cursor = database.queryFinalized("SELECT flags FROM dialog_settings WHERE did = ?", dialogId);
            try {
                if (cursor.next()) {
                    JSONObject setting = new JSONObject();
                    setting.put("did", dialogId);
                    setting.put("flags", cursor.longValue(0));
                    settings.put(setting);
                }
            } finally {
                cursor.dispose();
            }
        }
        return settings;
    }

    private static JSONArray buildUsersJson(SQLiteDatabase database, ArrayList<Long> userIds) throws Exception {
        JSONArray users = new JSONArray();
        for (long userId : userIds) {
            SQLiteCursor cursor = database.queryFinalized("SELECT data, status, name FROM users WHERE uid = ?", userId);
            try {
                if (cursor.next()) {
                    JSONObject user = new JSONObject();
                    user.put("uid", userId);
                    putBase64(user, "data", getBufferBytes(cursor.byteBufferValue(0)));
                    user.put("status", cursor.intValue(1));
                    user.put("name", cursor.stringValue(2));
                    users.put(user);
                }
            } finally {
                cursor.dispose();
            }
        }
        return users;
    }

    @Nullable
    private static JSONObject buildParamsJson(SQLiteDatabase database) throws Exception {
        SQLiteCursor cursor = database.queryFinalized("SELECT lsv, sg, pbytes FROM params WHERE id = 1");
        try {
            if (!cursor.next()) {
                return null;
            }
            JSONObject params = new JSONObject();
            params.put("lsv", cursor.intValue(0));
            params.put("sg", cursor.intValue(1));
            putBase64(params, "pbytes", cursor.byteArrayValue(2));
            return params;
        } finally {
            cursor.dispose();
        }
    }

    private static JSONArray buildRequestedHolesJson(SQLiteDatabase database, ArrayList<Integer> chatIds) throws Exception {
        JSONArray holes = new JSONArray();
        for (int chatId : chatIds) {
            JSONObject chatHoles = new JSONObject();
            chatHoles.put("uid", chatId);
            JSONArray rows = new JSONArray();

            SQLiteCursor cursor = database.queryFinalized("SELECT seq_out_start, seq_out_end FROM requested_holes WHERE uid = ?", chatId);
            try {
                while (cursor.next()) {
                    JSONObject row = new JSONObject();
                    row.put("start", cursor.intValue(0));
                    row.put("end", cursor.intValue(1));
                    rows.put(row);
                }
            } finally {
                cursor.dispose();
            }

            chatHoles.put("holes", rows);
            holes.put(chatHoles);
        }
        return holes;
    }

    private static void readEncryptedChatsJson(SQLiteDatabase database, @Nullable JSONArray chats, RestoreAccumulator accumulator) throws Exception {
        if (chats == null) {
            return;
        }

        SQLitePreparedStatement insert = database.executeFast("INSERT INTO enc_chats VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)");
        SQLitePreparedStatement update = database.executeFast("UPDATE enc_chats SET user=?, name=?, data=?, g=?, authkey=?, ttl=?, layer=?, seq_in=?, seq_out=?, use_count=?, exchange_id=?, key_date=?, fprint=?, fauthkey=?, khash=?, in_seq_no=?, admin_id=?, mtproto_seq=? WHERE uid=?");
        try {
            for (int i = 0; i < chats.length(); i++) {
                JSONObject chat = chats.getJSONObject(i);
                int chatId = chat.getInt("uid");
                int keyDate = chat.getInt("key_date");
                int seqOut = chat.getInt("seq_out");
                boolean exists = encryptedChatExists(database, chatId);
                boolean shouldWrite = shouldReplaceEncryptedChat(database, chatId, keyDate, seqOut);
                if (!exists) {
                    addUnique(accumulator.createdChatIds, chatId);
                }
                if (!shouldWrite) {
                    addUnique(accumulator.restoredChatIds, chatId);
                    continue;
                }

                SQLitePreparedStatement state = exists ? update : insert;
                state.requery();
                int position = 1;
                if (!exists) {
                    state.bindInteger(position++, chatId);
                }
                state.bindLong(position++, chat.getLong("user"));
                state.bindString(position++, chat.optString("name", ""));
                bindBlob(state, position++, fromBase64(chat, "data"));
                bindBlob(state, position++, fromBase64(chat, "g"));
                bindBlob(state, position++, fromBase64(chat, "authkey"));
                state.bindInteger(position++, chat.optInt("ttl", 0));
                state.bindInteger(position++, chat.optInt("layer", 0));
                state.bindInteger(position++, chat.optInt("seq_in", 0));
                state.bindInteger(position++, seqOut);
                state.bindInteger(position++, chat.optInt("use_count", 0));
                state.bindLong(position++, chat.optLong("exchange_id", 0));
                state.bindInteger(position++, keyDate);
                state.bindLong(position++, chat.optLong("fprint", 0));
                bindBlob(state, position++, fromBase64(chat, "fauthkey"));
                bindBlob(state, position++, fromBase64(chat, "khash"));
                state.bindInteger(position++, chat.optInt("in_seq_no", 0));
                state.bindLong(position++, chat.optLong("admin_id", 0));
                state.bindInteger(position++, chat.optInt("mtproto_seq", 0));
                if (exists) {
                    state.bindInteger(position, chatId);
                }
                state.step();
                addUnique(accumulator.restoredChatIds, chatId);
            }
        } finally {
            insert.dispose();
            update.dispose();
        }
    }

    private static void readMessagesJson(SQLiteDatabase database, @Nullable JSONArray chats, RestoreAccumulator accumulator) throws Exception {
        if (chats == null) {
            return;
        }

        SQLitePreparedStatement stateMsg = database.executeFast("INSERT OR IGNORE INTO messages_v2 VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)");
        SQLitePreparedStatement stateSeq = database.executeFast("INSERT OR IGNORE INTO messages_seq VALUES(?, ?, ?)");
        SQLitePreparedStatement stateRnd = database.executeFast("INSERT OR IGNORE INTO randoms_v2 VALUES(?, ?, ?)");
        SQLitePreparedStatement stateMed = database.executeFast("INSERT OR IGNORE INTO media_v4 VALUES(?, ?, ?, ?, ?)");
        try {
            for (int i = 0; i < chats.length(); i++) {
                JSONObject chat = chats.getJSONObject(i);
                int chatId = chat.getInt("uid");
                long dialogId = DialogObject.makeEncryptedDialogId(chatId);
                addUnique(accumulator.restoredChatIds, chatId);
                addUnique(accumulator.restoredDialogIds, dialogId);

                JSONArray messages = chat.optJSONArray("messages");
                if (messages == null) {
                    continue;
                }
                for (int j = 0; j < messages.length(); j++) {
                    JSONObject message = messages.getJSONObject(j);
                    int mid = message.getInt("mid");

                    byte[] mediaData = fromBase64(message, "med_data");
                    if (mediaData != null) {
                        stateMed.requery();
                        stateMed.bindInteger(1, mid);
                        stateMed.bindLong(2, dialogId);
                        stateMed.bindInteger(3, message.optInt("med_date", 0));
                        stateMed.bindInteger(4, message.optInt("med_type", 0));
                        bindBlob(stateMed, 5, mediaData);
                        stateMed.step();
                    }

                    stateMsg.requery();
                    stateMsg.bindInteger(1, mid);
                    stateMsg.bindLong(2, dialogId);
                    stateMsg.bindInteger(3, message.optInt("read_state", 0));
                    stateMsg.bindInteger(4, message.optInt("send_state", 0));
                    stateMsg.bindInteger(5, message.optInt("date", 0));
                    bindBlob(stateMsg, 6, fromBase64(message, "data"));
                    stateMsg.bindInteger(7, message.optInt("out", 0));
                    stateMsg.bindInteger(8, message.optInt("ttl", 0));
                    stateMsg.bindInteger(9, message.optInt("media", 0));
                    bindBlob(stateMsg, 10, fromBase64(message, "replydata"));
                    stateMsg.bindInteger(11, message.optInt("imp", 0));
                    stateMsg.bindInteger(12, message.optInt("mention", 0));
                    stateMsg.bindInteger(13, message.optInt("forwards", 0));
                    bindBlob(stateMsg, 14, fromBase64(message, "replies_data"));
                    stateMsg.bindInteger(15, message.optInt("thread_reply_id", 0));
                    stateMsg.bindInteger(16, message.optInt("is_channel", 0));
                    stateMsg.bindInteger(17, message.optInt("reply_to_message_id", 0));
                    bindBlob(stateMsg, 18, fromBase64(message, "custom_params"));
                    stateMsg.bindLong(19, message.optLong("group_id", 0));
                    stateMsg.bindInteger(20, message.optInt("reply_to_story_id", 0));
                    stateMsg.step();

                    int seqIn = message.optInt("seq_in", 0);
                    int seqOut = message.optInt("seq_out", 0);
                    if (seqIn != 0 || seqOut != 0) {
                        stateSeq.requery();
                        stateSeq.bindInteger(1, mid);
                        stateSeq.bindInteger(2, seqIn);
                        stateSeq.bindInteger(3, seqOut);
                        stateSeq.step();
                    }

                    long randomId = message.optLong("random_id", 0);
                    if (randomId != 0) {
                        stateRnd.requery();
                        stateRnd.bindLong(1, randomId);
                        stateRnd.bindInteger(2, mid);
                        stateRnd.bindLong(3, dialogId);
                        stateRnd.step();
                    }
                }
            }
        } finally {
            stateMsg.dispose();
            stateSeq.dispose();
            stateRnd.dispose();
            stateMed.dispose();
        }
    }

    private static void readDialogsJson(SQLiteDatabase database, @Nullable JSONArray dialogs, RestoreAccumulator accumulator) throws Exception {
        if (dialogs == null) {
            return;
        }

        SQLitePreparedStatement insert = database.executeFast("INSERT INTO dialogs VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)");
        SQLitePreparedStatement update = database.executeFast("UPDATE dialogs SET date=?, unread_count=?, last_mid=?, inbox_max=?, outbox_max=?, last_mid_i=?, unread_count_i=?, pts=?, date_i=?, pinned=?, flags=?, folder_id=?, data=?, unread_reactions=?, last_mid_group=?, ttl_period=?, unread_poll_votes=? WHERE did=?");
        try {
            for (int i = 0; i < dialogs.length(); i++) {
                JSONObject dialog = dialogs.getJSONObject(i);
                long dialogId = dialog.getLong("did");
                int date = dialog.optInt("date", 0);
                int lastMid = dialog.optInt("last_mid", 0);
                addUnique(accumulator.restoredDialogIds, dialogId);

                boolean exists = dialogExists(database, dialogId);
                if (exists && !shouldReplaceDialog(database, dialogId, date, lastMid)) {
                    continue;
                }

                SQLitePreparedStatement state = exists ? update : insert;
                state.requery();
                int position = 1;
                if (!exists) {
                    state.bindLong(position++, dialogId);
                }
                state.bindInteger(position++, date);
                state.bindInteger(position++, dialog.optInt("unread_count", 0));
                state.bindInteger(position++, lastMid);
                state.bindInteger(position++, dialog.optInt("inbox_max", 0));
                state.bindInteger(position++, dialog.optInt("outbox_max", 0));
                state.bindLong(position++, dialog.optLong("last_mid_i", 0));
                state.bindInteger(position++, dialog.optInt("unread_count_i", 0));
                state.bindInteger(position++, dialog.optInt("pts", 0));
                state.bindInteger(position++, dialog.optInt("date_i", 0));
                state.bindInteger(position++, dialog.optInt("pinned", 0));
                state.bindInteger(position++, dialog.optInt("flags", 0));
                state.bindInteger(position++, dialog.optInt("folder_id", 0));
                bindBlob(state, position++, fromBase64(dialog, "data"));
                state.bindInteger(position++, dialog.optInt("unread_reactions", 0));
                state.bindLong(position++, dialog.optLong("last_mid_group", 0));
                state.bindInteger(position++, dialog.optInt("ttl_period", 0));
                state.bindInteger(position++, dialog.optInt("unread_poll_votes", 0));
                if (exists) {
                    state.bindLong(position, dialogId);
                }
                state.step();
            }
        } finally {
            insert.dispose();
            update.dispose();
        }
    }

    private static void readDialogSettingsJson(SQLiteDatabase database, @Nullable JSONArray dialogSettings) throws Exception {
        if (dialogSettings == null) {
            return;
        }

        SQLitePreparedStatement state = database.executeFast("REPLACE INTO dialog_settings VALUES(?, ?)");
        try {
            for (int i = 0; i < dialogSettings.length(); i++) {
                JSONObject setting = dialogSettings.getJSONObject(i);
                state.requery();
                state.bindLong(1, setting.getLong("did"));
                state.bindLong(2, setting.optLong("flags", 0));
                state.step();
            }
        } finally {
            state.dispose();
        }
    }

    private static void readUsersJson(SQLiteDatabase database, @Nullable JSONArray users) throws Exception {
        if (users == null) {
            return;
        }

        SQLitePreparedStatement state = database.executeFast("REPLACE INTO users VALUES(?, ?, ?, ?)");
        try {
            for (int i = 0; i < users.length(); i++) {
                JSONObject user = users.getJSONObject(i);
                state.requery();
                state.bindLong(1, user.getLong("uid"));
                state.bindString(2, user.optString("name", ""));
                state.bindInteger(3, user.optInt("status", 0));
                bindBlob(state, 4, fromBase64(user, "data"));
                state.step();
            }
        } finally {
            state.dispose();
        }
    }

    private static void readParamsJson(SQLiteDatabase database, @Nullable JSONObject params) throws Exception {
        if (params == null) {
            return;
        }

        SQLitePreparedStatement state = database.executeFast("UPDATE params SET lsv = ?, sg = ?, pbytes = ? WHERE id = 1");
        try {
            state.requery();
            state.bindInteger(1, params.optInt("lsv", 0));
            state.bindInteger(2, params.optInt("sg", 0));
            bindBlob(state, 3, fromBase64(params, "pbytes"));
            state.step();
        } finally {
            state.dispose();
        }
    }

    private static void readRequestedHolesJson(SQLiteDatabase database, @Nullable JSONArray holes) throws Exception {
        if (holes == null) {
            return;
        }

        SQLitePreparedStatement state = database.executeFast("INSERT OR IGNORE INTO requested_holes VALUES(?, ?, ?)");
        try {
            for (int i = 0; i < holes.length(); i++) {
                JSONObject chatHoles = holes.getJSONObject(i);
                int chatId = chatHoles.getInt("uid");
                JSONArray rows = chatHoles.optJSONArray("holes");
                if (rows == null) {
                    continue;
                }
                for (int j = 0; j < rows.length(); j++) {
                    JSONObject row = rows.getJSONObject(j);
                    state.requery();
                    state.bindInteger(1, chatId);
                    state.bindInteger(2, row.optInt("start", 0));
                    state.bindInteger(3, row.optInt("end", 0));
                    state.step();
                }
            }
        } finally {
            state.dispose();
        }
    }

    private static boolean encryptedChatExists(SQLiteDatabase database, int chatId) throws Exception {
        SQLiteCursor cursor = database.queryFinalized("SELECT 1 FROM enc_chats WHERE uid = ?", chatId);
        try {
            return cursor.next();
        } finally {
            cursor.dispose();
        }
    }

    private static boolean dialogExists(SQLiteDatabase database, long dialogId) throws Exception {
        SQLiteCursor cursor = database.queryFinalized("SELECT 1 FROM dialogs WHERE did = ?", dialogId);
        try {
            return cursor.next();
        } finally {
            cursor.dispose();
        }
    }

    private static boolean shouldReplaceEncryptedChat(SQLiteDatabase database, int chatId, int keyDate, int seqOut) throws Exception {
        SQLiteCursor cursor = database.queryFinalized("SELECT key_date, seq_out FROM enc_chats WHERE uid = ?", chatId);
        try {
            if (!cursor.next()) {
                return true;
            }
            int localKeyDate = cursor.intValue(0);
            int localSeqOut = cursor.intValue(1);
            return keyDate > localKeyDate || keyDate == localKeyDate && seqOut >= localSeqOut;
        } finally {
            cursor.dispose();
        }
    }

    private static boolean shouldReplaceDialog(SQLiteDatabase database, long dialogId, int date, int lastMid) throws Exception {
        SQLiteCursor cursor = database.queryFinalized("SELECT date, last_mid FROM dialogs WHERE did = ?", dialogId);
        try {
            if (!cursor.next()) {
                return true;
            }
            int localDate = cursor.intValue(0);
            int localLastMid = cursor.intValue(1);
            return date > localDate || date == localDate && Math.abs(lastMid) >= Math.abs(localLastMid);
        } finally {
            cursor.dispose();
        }
    }

    private static void rehydrateRestoredDialogs(int currentAccount, MessagesStorage storage, RestoreAccumulator accumulator, StringBuilder logs) {
        if (accumulator.restoredDialogIds.isEmpty()) {
            AndroidUtilities.runOnUIThread(() -> NotificationCenter.getInstance(currentAccount).postNotificationName(NotificationCenter.dialogsNeedReload));
            return;
        }

        try {
            ArrayList<TLRPC.EncryptedChat> encryptedChats = new ArrayList<>();
            ArrayList<TLRPC.User> users = new ArrayList<>();
            ArrayList<TLRPC.Chat> chats = new ArrayList<>();
            TLRPC.messages_Dialogs dialogs = storage.loadDialogsByIdsForCache(accumulator.restoredDialogIds, encryptedChats, users, chats);

            AndroidUtilities.runOnUIThread(() -> {
                MessagesController controller = MessagesController.getInstance(currentAccount);
                controller.processLoadedDialogs(dialogs, encryptedChats, null, 0, 0, dialogs.dialogs.size(), 1, false, false, true);
                for (int i = 0; i < accumulator.createdChatIds.size(); i++) {
                    TLRPC.EncryptedChat encryptedChat = controller.getEncryptedChat(accumulator.createdChatIds.get(i));
                    if (encryptedChat != null) {
                        NotificationCenter.getInstance(currentAccount).postNotificationName(NotificationCenter.encryptedChatCreated, encryptedChat);
                    }
                }
                NotificationCenter.getInstance(currentAccount).postNotificationName(NotificationCenter.dialogsNeedReload);
            });
        } catch (Exception e) {
            log(logs, "Restore cache reload failed, falling back to generic refresh: " + e.getMessage());
            FileLog.e(e);
            AndroidUtilities.runOnUIThread(() -> {
                MessagesController controller = MessagesController.getInstance(currentAccount);
                for (int i = 0; i < accumulator.restoredChatIds.size(); i++) {
                    controller.getEncryptedChatDB(accumulator.restoredChatIds.get(i), false);
                }
                NotificationCenter.getInstance(currentAccount).postNotificationName(NotificationCenter.dialogsNeedReload);
            });
        }
    }

    private static void bindBlob(SQLitePreparedStatement state, int index, @Nullable byte[] data) throws Exception {
        if (data == null || data.length == 0) {
            state.bindNull(index);
            return;
        }
        NativeByteBuffer buffer = new NativeByteBuffer(data.length);
        buffer.writeBytes(data);
        state.bindByteBuffer(index, buffer);
        buffer.reuse();
    }

    @Nullable
    private static byte[] getBufferBytes(@Nullable NativeByteBuffer data) {
        if (data == null) {
            return null;
        }
        try {
            byte[] bytes = new byte[data.limit()];
            data.readBytes(bytes, false);
            return bytes;
        } finally {
            data.reuse();
        }
    }

    private static void putBase64(JSONObject object, String key, @Nullable byte[] data) throws Exception {
        if (data != null && data.length > 0) {
            object.put(key, Base64.encodeToString(data, Base64.NO_WRAP));
        }
    }

    @Nullable
    private static byte[] fromBase64(JSONObject object, String key) {
        if (!object.has(key) || object.isNull(key)) {
            return null;
        }
        return Base64.decode(object.optString(key, ""), Base64.NO_WRAP);
    }

    private static <T> void addUnique(ArrayList<T> list, T value) {
        if (!list.contains(value)) {
            list.add(value);
        }
    }

    private static void finish(@Nullable BackupDelegate delegate, boolean success, @Nullable String error, StringBuilder logs) {
        if (delegate != null) {
            delegate.onFinish(success, error, logs.toString());
        }
    }

    private static void readFully(FileInputStream fis, byte[] buffer) throws Exception {
        int offset = 0;
        while (offset < buffer.length) {
            int read = fis.read(buffer, offset, buffer.length - offset);
            if (read < 0) {
                throw new IllegalStateException("Unexpected end of file");
            }
            offset += read;
        }
    }
}
