package com.panasonic.diga.engineering.ir

import android.annotation.SuppressLint
import android.content.Context
import android.hardware.ConsumerIrManager
import android.os.Build
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.lang.reflect.Method
import java.util.concurrent.TimeUnit

private const val LEGACY_IRDA_SERVICE = "irda"
private const val LEGACY_IRDA_WRITE_METHOD = "write_irsend"
private const val SYSFS_SEC_IR_SEND = "/sys/class/sec/sec_ir/ir_send"
private const val SYSFS_SEC_IR_RESULT = "/sys/class/sec/sec_ir/ir_send_result"
private const val SYSFS_SEC_IRDA_TEST = "/sys/class/sec/sec_ir/irda_test"
private const val PREFS_NAME = "panadiga_ir_config"
private const val KEY_TIMING_UNIT_OVERRIDE = "timing_unit_override_v1"
private const val KEY_REPEAT_COUNT = "repeat_count_v1"
private const val KEY_CUSTOM_PROFILES_JSON = "custom_keymap_profiles_v1"
private const val SU_PROBE_TIMEOUT_MS = 2500L
private const val SU_TX_TIMEOUT_MS = 5000L

/**
 * Slim Android IR hardware executor: delegates all Kaseikyo protocol math, Pronto Hex decoding,
 * and Exynos driver scaling to `panadiga-core::ir` in Rust, keeping Kotlin focused strictly on
 * kernel sysfs / Magisk `su` / legacy `irda` / AOSP `ConsumerIrManager` I/O and `SharedPreferences`.
 */
class AndroidIrExecutor(context: Context) {
    private val appContext: Context = context.applicationContext
    private val prefs = appContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    private val irManager: ConsumerIrManager? =
        runCatching {
            appContext.getSystemService(Context.CONSUMER_IR_SERVICE) as? ConsumerIrManager
        }.getOrNull()

    @SuppressLint("WrongConstant")
    private val irdaService: Any? =
        runCatching { appContext.getSystemService(LEGACY_IRDA_SERVICE) }.getOrNull()

    private val irdaWriteMethod: Method? =
        runCatching {
            irdaService?.javaClass?.getMethod(LEGACY_IRDA_WRITE_METHOD, String::class.java)
        }.getOrNull()

    private val sysfsIrSend = File(SYSFS_SEC_IR_SEND)
    private val sysfsIrdaTest = File(SYSFS_SEC_IRDA_TEST)

    @Volatile
    private var suUnlockAttempted: Boolean = false

    @Volatile
    private var suProbeCached: Boolean = false

    @Volatile
    private var suHasSecIr: Boolean = false

    @Volatile
    private var suHasFpga: Boolean = false

    @Volatile
    private var suBinaryDetected: Boolean = false

    private fun ensureSuSysfsProbed() {
        if (suProbeCached) return
        synchronized(this) {
            if (suProbeCached) return
            suProbeCached = true
            if (sysfsIrSend.exists() && sysfsIrdaTest.exists()) {
                suHasSecIr = true
                suHasFpga = true
                suBinaryDetected = probeSuBinaryAvailable()
                return
            }
            // Fallback probe for SELinux untrusted_app builds where File.exists() returns false on /sys/class/sec/sec_ir
            suBinaryDetected = runSuCommand("id >/dev/null 2>&1", SU_PROBE_TIMEOUT_MS)
            if (suBinaryDetected) {
                suHasSecIr = sysfsIrSend.exists() || runSuCommand("[ -e $SYSFS_SEC_IR_SEND ]", SU_PROBE_TIMEOUT_MS)
                suHasFpga = sysfsIrdaTest.exists() || runSuCommand("[ -e $SYSFS_SEC_IRDA_TEST ]", SU_PROBE_TIMEOUT_MS)
            }
        }
    }

    fun probeCapabilities(): JSONObject {
        ensureSuSysfsProbed()
        val aospEmitter = runCatching { irManager?.hasIrEmitter() == true }.getOrDefault(false)
        val hasSecIr = sysfsIrSend.exists() || suHasSecIr
        val hasFpga = sysfsIrdaTest.exists() || suHasFpga
        val ranges = readFrequencyRanges()
        val hasEmitter = aospEmitter || hasSecIr || irdaWriteMethod != null || ranges.length() > 0
        val serviceFound = irManager != null || irdaWriteMethod != null || hasSecIr

        val devObj = JSONObject()
            .put("manufacturer", Build.MANUFACTURER.orEmpty())
            .put("model", Build.MODEL.orEmpty())
            .put("hardware", Build.HARDWARE.orEmpty())
            .put("board", Build.BOARD.orEmpty())
            .put("product", Build.PRODUCT.orEmpty())
            .put("device", Build.DEVICE.orEmpty())
            .put("android_release", Build.VERSION.RELEASE.orEmpty())
            .put("sdk_int", Build.VERSION.SDK_INT)

        return JSONObject()
            .put("hasEmitter", hasEmitter)
            .put("serviceFound", serviceFound)
            .put("hasSecIrNode", hasSecIr)
            .put("hasFpgaNode", hasFpga)
            .put("suAvailable", suBinaryDetected)
            .put("ranges", ranges)
            .put("device", devObj)
    }

    fun loadSettings(): JSONObject =
        JSONObject()
            .put("timingUnitOverride", prefs.getString(KEY_TIMING_UNIT_OVERRIDE, "") ?: "")
            .put("repeatCount", prefs.getInt(KEY_REPEAT_COUNT, 1).coerceIn(1, 3))
            .put("customProfilesJson", prefs.getString(KEY_CUSTOM_PROFILES_JSON, "[]") ?: "[]")

    fun saveSettings(timingUnitOverride: String, repeatCount: Int): JSONObject {
        val clampedRepeat = repeatCount.coerceIn(1, 3)
        prefs.edit()
            .putString(KEY_TIMING_UNIT_OVERRIDE, timingUnitOverride)
            .putInt(KEY_REPEAT_COUNT, clampedRepeat)
            .apply()
        return loadSettings()
    }

    fun saveCustomProfilesJson(profilesJson: String): Boolean {
        prefs.edit().putString(KEY_CUSTOM_PROFILES_JSON, profilesJson).apply()
        return true
    }

    fun transmitPrepared(
        carrierHz: Int,
        encodedPattern: IntArray,
        sysfsPayload: String,
        repeatCount: Int,
    ): Result<String> = runCatching {
        ensureSuSysfsProbed()
        val caps = probeCapabilities()
        if (!caps.optBoolean("hasEmitter", false)) {
            error("Consumer IR hardware unavailable on this device")
        }
        val repeats = repeatCount.coerceIn(1, 3)
        var lastBackend = "consumer_ir_manager"
        repeat(repeats) {
            lastBackend = transmitSingleFrame(carrierHz, encodedPattern, sysfsPayload)
        }
        lastBackend
    }

    private fun transmitSingleFrame(
        carrierHz: Int,
        encodedPattern: IntArray,
        sysfsPayload: String,
    ): String {
        if (tryWriteSysfsDirect(sysfsPayload)) return "sysfs_direct"

        if ((sysfsIrSend.exists() || suHasSecIr) && !sysfsIrSend.canWrite() && !suUnlockAttempted) {
            unlockSysfsViaSu()
            if (tryWriteSysfsDirect(sysfsPayload)) return "sysfs_su_unlock"
        }

        if ((sysfsIrSend.exists() || suHasSecIr) && tryWriteSysfsViaSu(sysfsPayload)) {
            return "sysfs_su"
        }

        if (tryWriteLegacyIrda(sysfsPayload)) return "legacy_irda"

        val manager = irManager ?: error("ConsumerIrManager unavailable and sysfs/su failed")
        manager.transmit(carrierHz, encodedPattern)
        return "consumer_ir_manager"
    }

    private fun tryWriteSysfsDirect(payload: String): Boolean =
        runCatching {
            if (!sysfsIrSend.exists() || !sysfsIrSend.canWrite()) return false
            sysfsIrSend.writeText(payload)
            true
        }.getOrDefault(false)

    private fun unlockSysfsViaSu() {
        suUnlockAttempted = true
        runSuCommand(
            "chmod 0666 $SYSFS_SEC_IR_SEND 2>/dev/null; " +
                "chown system:radio $SYSFS_SEC_IR_SEND 2>/dev/null; " +
                "chmod 0666 $SYSFS_SEC_IR_RESULT 2>/dev/null || true",
            SU_TX_TIMEOUT_MS,
        )
    }

    private fun tryWriteSysfsViaSu(payload: String): Boolean {
        val piped = runCatching {
            val process = ProcessBuilder("su").redirectErrorStream(true).start()
            process.outputStream.bufferedWriter().use { writer ->
                writer.write("echo '$payload' > $SYSFS_SEC_IR_SEND\n")
                writer.write("exit\n")
                writer.flush()
            }
            val finished = process.waitFor(SU_TX_TIMEOUT_MS, TimeUnit.MILLISECONDS)
            if (finished && process.exitValue() == 0) {
                true
            } else {
                process.destroy()
                false
            }
        }.getOrDefault(false)
        if (piped) return true
        return runSuCommand("echo '$payload' > $SYSFS_SEC_IR_SEND", SU_TX_TIMEOUT_MS)
    }

    private fun tryWriteLegacyIrda(payload: String): Boolean {
        val service = irdaService ?: return false
        val method = irdaWriteMethod ?: return false
        return runCatching {
            method.invoke(service, payload)
            true
        }.getOrDefault(false)
    }

    private fun runSuCommand(command: String, timeoutMs: Long): Boolean =
        runCatching {
            val process = ProcessBuilder("su", "-c", command).redirectErrorStream(true).start()
            val finished = process.waitFor(timeoutMs, TimeUnit.MILLISECONDS)
            if (!finished) {
                process.destroy()
                false
            } else {
                process.exitValue() == 0
            }
        }.getOrDefault(false)

    private fun probeSuBinaryAvailable(): Boolean =
        listOf("/system/bin/su", "/system/xbin/su", "/sbin/su", "/debug_ramdisk/su")
            .any { File(it).exists() }

    private fun readFrequencyRanges(): JSONArray {
        val arr = JSONArray()
        val raw = runCatching { irManager?.carrierFrequencies }.getOrNull().orEmpty()
        for (range in raw) {
            arr.put(JSONObject().put("min_hz", range.minFrequency).put("max_hz", range.maxFrequency))
        }
        return arr
    }
}
