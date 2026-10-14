package moe.hx030.momogram.util;

import android.text.TextUtils;
import android.util.Log;

import org.telegram.messenger.AndroidUtilities;
import org.telegram.messenger.ContactsController;
import org.telegram.messenger.MessageObject;
import org.telegram.messenger.MessagesController;
import org.telegram.messenger.MessagesStorage;
import org.telegram.messenger.NotificationsController;
import org.telegram.messenger.UserObject;
import org.telegram.messenger.Utilities;
import org.telegram.tgnet.TLObject;
import org.telegram.tgnet.TLRPC;

import java.util.ArrayList;
import java.util.regex.Pattern;

import moe.hx030.momogram.MomoConfig;
import moe.hx030.momogram.cc.CCConverter;
import moe.hx030.momogram.cc.CCTarget;
import moe.hx030.momogram.utils.RustCore;

public class FilterUtils {

    private static final int NATIVE_BLOCKED = 0;
    private static final int NATIVE_PASSED = 1;
    private static final int NATIVE_PENDING = 2;
    private static final int NATIVE_ARCHIVE = 3;
    private static final int NATIVE_LOAD_FULL = 4;

    public enum Result {
        Blocked,
        Passed,
        Pending
    }

    public static Result filterPM(int currentAccount, MessageObject messageObject, TLObject diff) {
        return filterPM(currentAccount, messageObject.getChatId(), messageObject.getSenderId(), messageObject.isFromUser(), diff);
    }

    public static Result filterPM(int currentAccount, Long chatId, Long senderId, Boolean isFromUser, TLObject diff) {
        if (senderId == null) return Result.Passed;
        if (chatId == null) chatId = 0L;
        if (isFromUser == null) isFromUser = true;

        ContactsController contactsController = ContactsController.getInstance(currentAccount);
        MessagesController messagesController = MessagesController.getInstance(currentAccount);
        MessagesStorage messagesStorage = MessagesStorage.getInstance(currentAccount);
        NotificationsController notificationsController = NotificationsController.getInstance(currentAccount);

        boolean blocked = messagesController.blockedPeers.indexOfKey(senderId) >= 0;
        int precheck = RustCore.filterPrecheck(
                MomoConfig.ignoreBlocked.Bool(),
                blocked,
                MomoConfig.autoArchiveAndMute.Bool(),
                senderId,
                chatId,
                isFromUser
        );
        if (precheck == NATIVE_BLOCKED) return Result.Blocked;
        if (precheck == NATIVE_PASSED) return Result.Passed;

        boolean service = UserObject.isService(senderId);
        boolean contact = contactsController.isContact(senderId);

        if (MomoConfig.debugAntiSpam.Bool()) {
            Log.d("030-debugspam", String.format(
                    "PM?, id=%d, chat=%d, isSvc=%s, isContact=%s",
                    senderId, chatId, service, contact
            ));
        }

        TLRPC.User currentUser = messagesStorage.getUserSync(chatId);
        boolean existingChat = false;

        if (!service && !contact) {
            existingChat = messagesStorage.isExistingChat(senderId);
            if (!existingChat && currentUser == null && diff != null) {
                try {
                    Object us = ReflectUtil.getFieldValue(diff, "users");
                    if (us instanceof ArrayList<?> arr) {
                        for (Object item : arr) {
                            if (item instanceof TLRPC.User user && user.id == senderId) {
                                currentUser = user;
                                break;
                            }
                        }
                    }
                } catch (Exception ex) {
                    Log.e(
                            "030-filter",
                            String.format("failed to get %d from current update data, skipping", senderId),
                            ex
                    );
                }
            }
        }

        int decision = RustCore.filterDecide(
                senderId,
                service,
                contact,
                existingChat,
                currentUser != null,
                currentUser != null && currentUser.bot,
                MomoConfig.autoArchiveAndMuteNoCommonGroupOnly.Bool()
        );

        if (decision == NATIVE_PENDING) {
            if (service || contact) {
                Log.e(
                        "030-filter",
                        String.format("cannot fetch currentUser for %d %d, skipping...", senderId, chatId)
                );
            }
            return Result.Pending;
        }
        if (decision == NATIVE_PASSED) return Result.Passed;
        if (decision == NATIVE_BLOCKED) return Result.Blocked;

        final ArrayList<Long> list = new ArrayList<>(1);
        list.add(senderId);

        if (decision == NATIVE_LOAD_FULL && currentUser != null) {
            TLRPC.User user = currentUser;
            messagesController.loadFullUser(user, messagesStorage.classGuid, true, uf -> {
                if (uf != null && uf.common_chats_count > 0) return;
                Log.d("030-spam", "no common group => archive & mute " + senderId);
                archiveAndMute(messagesController, notificationsController, list, senderId);
            });
        } else if (decision == NATIVE_ARCHIVE) {
            Log.d("030-spam", "archive & mute " + senderId);
            archiveAndMute(messagesController, notificationsController, list, senderId);
        }

        return Result.Passed;
    }

    public static void checkPendingIds(int currentAccount) {
        if (RustCore.pendingIsEmpty()) return;
        long[] pending = RustCore.takePendingIds();
        Log.d("030-filter", String.format("checkPendingIds: count=%d", pending.length));
        Utilities.stageQueue.postRunnable(() -> {
            for (long id : pending) {
                filterPM(currentAccount, null, id, null, null);
            }
        });
    }

    public static void archiveAndMute(
            MessagesController messagesController,
            NotificationsController notificationsController,
            ArrayList<Long> list,
            long id
    ) {
        AndroidUtilities.runOnUIThread(() -> {
            messagesController.addDialogToFolder(list, 1, -1, null, 0);
            notificationsController.setDialogNotificationsSettings(
                    id,
                    0,
                    NotificationsController.SETTING_MUTE_FOREVER
            );
        });
    }

    public static boolean checkName(Pattern regex, String firstname, String lastname, boolean useOpenCC) {
        if (regex == null) return false;
        if (matches(regex, firstname, useOpenCC)) return true;
        return !TextUtils.isEmpty(lastname) && matches(regex, lastname, useOpenCC);
    }

    public static boolean checkString(Pattern regex, String str, boolean useOpenCC) {
        if (regex == null || TextUtils.isEmpty(str)) return false;
        return matches(regex, str, useOpenCC);
    }

    private static boolean matches(Pattern regex, String text, boolean useOpenCC) {
        int result = RustCore.filterMatches(regex.pattern(), regex.flags(), text, useOpenCC);
        if (result >= 0) return result == 1;

        if (regex.matcher(text).find()) return true;
        if (useOpenCC) {
            for (CCTarget target : new CCTarget[]{CCTarget.TC, CCTarget.SC}) {
                if (regex.matcher(CCConverter.get(target).convert(text)).find()) return true;
            }
        }
        return false;
    }
}
