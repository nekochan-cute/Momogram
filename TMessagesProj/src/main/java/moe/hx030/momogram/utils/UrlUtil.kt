package moe.hx030.momogram.utils

import android.net.Uri
import moe.hx030.momogram.MomoConfig

object UrlUtil {
    @JvmStatic
    fun isTwitter(url: String): Boolean = RustCore.isTwitter(url)

    @JvmStatic
    fun isTwitterPreview(url: String): Boolean = RustCore.isTwitterPreview(url)

    @JvmStatic
    fun cleanUrl(src: String): String = RustCore.cleanUrl(
        src,
        MomoConfig.patchAndCleanupLinks.Bool(),
        MomoConfig.customGetQueryBlacklistData.joinToString("\n")
    )

    @JvmStatic
    fun cleanUrl(src: Uri): Uri = Uri.parse(cleanUrl(src.toString()))

    @JvmStatic
    fun stripAllQueries(src: String): String = RustCore.stripAllQueries(src)
}
