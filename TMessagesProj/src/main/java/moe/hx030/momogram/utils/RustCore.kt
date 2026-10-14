package moe.hx030.momogram.utils

object RustCore {
    init {
        System.loadLibrary("momogram_core")
    }

    @JvmStatic
    fun cleanUrl(src: String, patchLinks: Boolean, customBlacklist: String): String =
        nativeCleanUrl(src, patchLinks, customBlacklist)

    @JvmStatic
    fun isTwitter(host: String): Boolean = nativeIsTwitter(host)

    @JvmStatic
    fun isTwitterPreview(host: String): Boolean = nativeIsTwitterPreview(host)

    @JvmStatic
    fun stripAllQueries(src: String): String = nativeStripAllQueries(src)

    @JvmStatic
    fun openCcConvert(target: Int, input: String): String = nativeOpenCcConvert(target, input)

    @JvmStatic
    fun getSubString(text: String, left: String?, right: String?, last: Boolean): String =
        nativeGetSubString(text, left, right, last)

    @JvmStatic
    fun isInteger(value: String): Boolean = nativeIsInteger(value)

    @JvmStatic
    fun replaceAllRegex(content: String, pattern: String, replacement: String?): String =
        nativeReplaceAllRegex(content, pattern, replacement)

    @JvmStatic
    fun escape(content: String): String = nativeEscape(content)

    @JvmStatic
    fun simpleUuid(): String = nativeSimpleUuid()

    @JvmStatic
    fun isRtl(value: String?): Boolean = nativeIsRtl(value)

    @JvmStatic
    fun firstCharUpper(value: String?): String? = nativeFirstCharUpper(value)

    @JvmStatic
    fun prefix030(simpleName: String): String = nativePrefix030(simpleName)

    @JvmStatic
    fun selectName(useOld: Boolean, old: String, current: String): String = nativeSelectName(useOld, old, current)

    @JvmStatic
    fun isAppName(value: String, a: String, b: String, c: String): Boolean = nativeIsAppName(value, a, b, c)

    @JvmStatic
    fun appendCsv(original: String?, addition: String): String = nativeAppendCsv(original, addition)

    @JvmStatic
    fun randomizeFileName(name: String): String = nativeRandomizeFileName(name)

    @JvmStatic
    fun mentionRanges(text: String): IntArray = nativeMentionRanges(text)

    @JvmStatic
    fun filterPrecheck(
        ignoreBlocked: Boolean,
        isBlocked: Boolean,
        autoArchive: Boolean,
        senderId: Long,
        chatId: Long,
        isFromUser: Boolean
    ): Int = nativeFilterPrecheck(ignoreBlocked, isBlocked, autoArchive, senderId, chatId, isFromUser)

    @JvmStatic
    fun filterDecide(
        senderId: Long,
        isService: Boolean,
        isContact: Boolean,
        existingChat: Boolean,
        currentUserPresent: Boolean,
        currentUserIsBot: Boolean,
        noCommonGroupOnly: Boolean
    ): Int = nativeFilterDecide(
        senderId,
        isService,
        isContact,
        existingChat,
        currentUserPresent,
        currentUserIsBot,
        noCommonGroupOnly
    )

    @JvmStatic
    fun bumpPendingId(senderId: Long) = nativeBumpPendingId(senderId)

    @JvmStatic
    fun pendingIsEmpty(): Boolean = nativePendingIsEmpty()

    @JvmStatic
    fun takePendingIds(): LongArray = nativeTakePendingIds()

    @JvmStatic
    fun filterRegexValid(pattern: String): Boolean = nativeFilterRegexValid(pattern)

    @JvmStatic
    fun filterMatches(pattern: String, flags: Int, text: String, useOpenCc: Boolean): Int =
        nativeFilterMatches(pattern, flags, text, useOpenCc)

    @JvmStatic
    fun parseProxyUri(raw: String): String? = nativeParseProxyUri(raw)

    @JvmStatic
    fun proxyNormalizeWebAddress(address: String?): String = nativeProxyNormalizeWebAddress(address)

    @JvmStatic
    fun proxyIsValidWebAddress(address: String?): Boolean = nativeProxyIsValidWebAddress(address)

    @JvmStatic
    fun proxyIsValidWebSecret(secret: String?): Boolean = nativeProxyIsValidWebSecret(secret)

    @JvmStatic
    fun proxyIsValidWebPath(path: String): Boolean = nativeProxyIsValidWebPath(path)

    @JvmStatic
    fun proxyDecodeMarkedSecret(secret: String?): String? = nativeProxyDecodeMarkedSecret(secret)

    @JvmStatic
    fun proxyEncodeMarkedSecret(secret: String?): String? = nativeProxyEncodeMarkedSecret(secret)

    @JvmStatic
    fun encodeSecretBackup(password: String, format: Int, stateJson: String): ByteArray =
        nativeEncodeSecretBackup(password, format, stateJson)

    @JvmStatic
    fun decodeSecretBackup(password: String, file: ByteArray): String =
        nativeDecodeSecretBackup(password, file)

    @JvmStatic private external fun nativeCleanUrl(src: String, patchLinks: Boolean, customBlacklist: String): String
    @JvmStatic private external fun nativeIsTwitter(host: String): Boolean
    @JvmStatic private external fun nativeIsTwitterPreview(host: String): Boolean
    @JvmStatic private external fun nativeStripAllQueries(src: String): String
    @JvmStatic private external fun nativeOpenCcConvert(target: Int, input: String): String
    @JvmStatic private external fun nativeGetSubString(text: String, left: String?, right: String?, last: Boolean): String
    @JvmStatic private external fun nativeIsInteger(value: String): Boolean
    @JvmStatic private external fun nativeReplaceAllRegex(content: String, pattern: String, replacement: String?): String
    @JvmStatic private external fun nativeEscape(content: String): String
    @JvmStatic private external fun nativeSimpleUuid(): String
    @JvmStatic private external fun nativeIsRtl(value: String?): Boolean
    @JvmStatic private external fun nativeFirstCharUpper(value: String?): String?
    @JvmStatic private external fun nativePrefix030(simpleName: String): String
    @JvmStatic private external fun nativeSelectName(useOld: Boolean, old: String, current: String): String
    @JvmStatic private external fun nativeIsAppName(value: String, a: String, b: String, c: String): Boolean
    @JvmStatic private external fun nativeAppendCsv(original: String?, addition: String): String
    @JvmStatic private external fun nativeRandomizeFileName(name: String): String
    @JvmStatic private external fun nativeMentionRanges(text: String): IntArray
    @JvmStatic private external fun nativeFilterPrecheck(ignoreBlocked: Boolean, isBlocked: Boolean, autoArchive: Boolean, senderId: Long, chatId: Long, isFromUser: Boolean): Int
    @JvmStatic private external fun nativeFilterDecide(senderId: Long, isService: Boolean, isContact: Boolean, existingChat: Boolean, currentUserPresent: Boolean, currentUserIsBot: Boolean, noCommonGroupOnly: Boolean): Int
    @JvmStatic private external fun nativeBumpPendingId(senderId: Long)
    @JvmStatic private external fun nativePendingIsEmpty(): Boolean
    @JvmStatic private external fun nativeTakePendingIds(): LongArray
    @JvmStatic private external fun nativeFilterRegexValid(pattern: String): Boolean
    @JvmStatic private external fun nativeFilterMatches(pattern: String, flags: Int, text: String, useOpenCc: Boolean): Int
    @JvmStatic private external fun nativeParseProxyUri(raw: String): String?
    @JvmStatic private external fun nativeProxyNormalizeWebAddress(address: String?): String
    @JvmStatic private external fun nativeProxyIsValidWebAddress(address: String?): Boolean
    @JvmStatic private external fun nativeProxyIsValidWebSecret(secret: String?): Boolean
    @JvmStatic private external fun nativeProxyIsValidWebPath(path: String): Boolean
    @JvmStatic private external fun nativeProxyDecodeMarkedSecret(secret: String?): String?
    @JvmStatic private external fun nativeProxyEncodeMarkedSecret(secret: String?): String?
    @JvmStatic private external fun nativeEncodeSecretBackup(password: String, format: Int, stateJson: String): ByteArray
    @JvmStatic private external fun nativeDecodeSecretBackup(password: String, file: ByteArray): String
}
