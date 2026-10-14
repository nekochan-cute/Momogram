package moe.hx030.momogram.cc

import moe.hx030.momogram.utils.RustCore
import org.telegram.messenger.ApplicationLoader
import java.io.ByteArrayOutputStream
import java.io.DataOutputStream
import java.util.concurrent.atomic.AtomicBoolean

class CCConverter private constructor(val target: CCTarget) {

    companion object {
        private val instances = HashMap<CCTarget, CCConverter>()
        private val initialized = AtomicBoolean(false)
        private val initLock = Any()

        @JvmStatic
        fun get(target: CCTarget): CCConverter {
            ensureRustOpenCc()
            return instances[target] ?: synchronized(instances) {
                instances[target] ?: CCConverter(target).also {
                    instances[target] = it
                }
            }
        }

        private fun ensureRustOpenCc() {
            if (initialized.get()) return
            synchronized(initLock) {
                if (initialized.get()) return

                val assets = ApplicationLoader.applicationContext.assets
                val buffer = ByteArrayOutputStream()
                DataOutputStream(buffer).use { out ->
                    val dicts = CCDict.entries
                    out.writeInt(dicts.size)
                    for (dict in dicts) {
                        val name = dict.name.toByteArray(Charsets.UTF_8)
                        val content = assets.open("dictionary/${dict.name}.txt").use { it.readBytes() }
                        out.writeInt(name.size)
                        out.write(name)
                        out.writeInt(content.size)
                        out.write(content)
                    }
                }
                check(RustCore.initOpenCc(buffer.toByteArray())) {
                    "Rust OpenCC initialization failed"
                }
                initialized.set(true)
            }
        }
    }

    fun convert(input: String): String = RustCore.openCcConvert(target.ordinal, input)
}
