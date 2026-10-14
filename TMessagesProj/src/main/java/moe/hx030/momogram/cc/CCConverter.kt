package moe.hx030.momogram.cc

import moe.hx030.momogram.utils.RustCore

class CCConverter private constructor(val target: CCTarget) {

    companion object {
        private val instances = HashMap<CCTarget, CCConverter>()

        @JvmStatic
        fun get(target: CCTarget): CCConverter {
            return instances[target] ?: synchronized(instances) {
                instances[target] ?: CCConverter(target).also {
                    instances[target] = it
                }
            }
        }
    }

    fun convert(input: String): String = RustCore.openCcConvert(target.ordinal, input)
}
