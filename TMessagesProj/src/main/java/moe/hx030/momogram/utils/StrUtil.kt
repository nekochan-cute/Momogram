package moe.hx030.momogram.utils

import android.text.SpannableStringBuilder
import android.view.View
import android.widget.TextView
import org.telegram.messenger.AndroidUtilities
import org.telegram.messenger.FileLog
import org.telegram.messenger.LocaleController
import org.telegram.messenger.R
import org.telegram.ui.ActionBar.BaseFragment
import org.telegram.ui.Components.URLSpanNoUnderline
import moe.hx030.momogram.MomoConfig
import moe.hx030.momogram.config.ConfigItem

object StrUtil {

    @JvmStatic
    fun setText(fragment: BaseFragment?, textView: TextView, text: String) {
        var stringBuilder: SpannableStringBuilder? = null
        if (fragment != null) {
            try {
                val ranges = RustCore.mentionRanges(text)
                var i = 0
                while (i + 1 < ranges.size) {
                    val start = ranges[i]
                    val end = ranges[i + 1]
                    if (stringBuilder == null) {
                        stringBuilder = SpannableStringBuilder(text)
                        textView.movementMethod = AndroidUtilities.LinkMovementMethodMy()
                    }
                    val username = text.subSequence(start + 1, end).toString()
                    val url = object : URLSpanNoUnderline(username) {
                        override fun onClick(widget: View) {
                            fragment.messagesController.openByUserName(url, fragment, 1)
                        }
                    }
                    stringBuilder.setSpan(url, start, end, 0)
                    i += 2
                }
            } catch (e: Exception) {
                FileLog.e(e)
            }
        }
        textView.text = stringBuilder ?: text
    }

    @JvmStatic
    @JvmOverloads
    fun getSubString(text: String, left: String?, right: String?, lastOrFirst: Boolean = false): String =
        RustCore.getSubString(text, left, right, lastOrFirst)

    @JvmStatic
    fun isInteger(str: String): Boolean = RustCore.isInteger(str)

    @JvmStatic
    fun replaceAllRegex(
        content: CharSequence,
        patternStr: String,
        replacementTemplate: String?
    ): String = RustCore.replaceAllRegex(content.toString(), patternStr, replacementTemplate)

    fun escape(content: String): String = RustCore.escape(content)

    @JvmStatic
    fun getSimpleUUID(): String = RustCore.simpleUuid()

    @JvmStatic
    fun isRTLString(str: String?): Boolean = RustCore.isRtl(str)

    @JvmStatic
    fun firstCharUpper(str: String?): String? = RustCore.firstCharUpper(str)

    @JvmStatic
    fun get030Tag(obj: Any): String = RustCore.prefix030(obj.javaClass.simpleName)

    val appNames = listOf(
        LocaleController.getString(R.string.NekoX),
        LocaleController.getString(R.string.AppNameShort),
        LocaleController.getString(R.string.Momogram),
    )

    @JvmStatic
    fun getAppName(): String = RustCore.selectName(MomoConfig.useOldName.Bool(), appNames[0], appNames[2])

    @JvmStatic
    fun getShortAppName(): String = RustCore.selectName(MomoConfig.useOldName.Bool(), appNames[1], appNames[2])

    @JvmStatic
    fun isAppName(s: String): Boolean = RustCore.isAppName(s, appNames[0], appNames[1], appNames[2])

    @JvmStatic
    fun appendToCSConfigString(cfg: ConfigItem, s: Collection<String>) {
        cfg.setConfigString(RustCore.appendCsv(cfg.String(), s.joinToString(",")))
    }

    @JvmStatic
    fun randomizeFileName(n: String): String {
        if (!MomoConfig.randomizeFilenameOnSend.Bool() || n.endsWith(".m0m0-crash.txt")) {
            return n
        }
        return RustCore.randomizeFileName(n)
    }
}
