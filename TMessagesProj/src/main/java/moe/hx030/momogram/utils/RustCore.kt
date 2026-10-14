package moe.hx030.momogram.utils

object RustCore {
    @Volatile
    private var loadState: Boolean? = null

    private fun ensureLoaded(): Boolean {
        loadState?.let { return it }
        return synchronized(this) {
            loadState?.let { return@synchronized it }
            val loaded = try {
                System.loadLibrary("momogram_core")
                true
            } catch (_: UnsatisfiedLinkError) {
                false
            }
            loadState = loaded
            loaded
        }
    }

    @JvmStatic
    fun cleanUrl(src: String, patchLinks: Boolean, customBlacklist: String): String? {
        if (!ensureLoaded()) return null
        return try {
            nativeCleanUrl(src, patchLinks, customBlacklist)
        } catch (_: UnsatisfiedLinkError) {
            loadState = false
            null
        }
    }

    @JvmStatic
    private external fun nativeCleanUrl(src: String, patchLinks: Boolean, customBlacklist: String): String
}
