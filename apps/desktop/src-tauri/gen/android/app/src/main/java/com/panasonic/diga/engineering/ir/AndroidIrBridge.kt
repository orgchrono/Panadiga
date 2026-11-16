package com.panasonic.diga.engineering.ir

import android.content.Context
import android.webkit.JavascriptInterface
import org.json.JSONArray
import org.json.JSONObject

/**
 * WebView JavascriptInterface (`window.__PANADIGA_IR_BRIDGE__`) exposing raw Android IR hardware I/O
 * to React/MVVM ViewModels while Rust (`panadiga-core::ir`) handles protocol math and catalogs.
 */
class AndroidIrBridge(context: Context) {
    private val executor = AndroidIrExecutor(context)

    @JavascriptInterface
    fun isAvailable(): Boolean = true

    @JavascriptInterface
    fun getCapabilitiesJson(): String =
        runCatching { executor.probeCapabilities().toString() }
            .getOrElse { err ->
                JSONObject()
                    .put("hasEmitter", false)
                    .put("serviceFound", false)
                    .put("error", err.message ?: "IR probe failure")
                    .toString()
            }

    @JavascriptInterface
    fun getSettingsJson(): String =
        runCatching { executor.loadSettings().toString() }
            .getOrElse { "{\"timingUnitOverride\":\"\",\"repeatCount\":1,\"customProfilesJson\":\"[]\"}" }

    @JavascriptInterface
    fun saveSettings(timingUnitOverride: String, repeatCount: Int): String =
        runCatching { executor.saveSettings(timingUnitOverride, repeatCount).toString() }
            .getOrElse { "{}" }

    @JavascriptInterface
    fun saveCustomProfilesJson(profilesJson: String): Boolean =
        runCatching { executor.saveCustomProfilesJson(profilesJson) }.getOrDefault(false)

    @JavascriptInterface
    fun transmitPreparedJson(
        carrierHz: Int,
        encodedPatternJson: String,
        sysfsPayload: String,
        repeatCount: Int,
    ): String {
        val res = JSONObject()
        val pattern = runCatching {
            val arr = JSONArray(encodedPatternJson)
            IntArray(arr.length()) { idx -> arr.getInt(idx) }
        }.getOrElse { err ->
            return res
                .put("ok", false)
                .put("error", "Invalid pattern JSON: ${err.message}")
                .toString()
        }

        val outcome = executor.transmitPrepared(carrierHz, pattern, sysfsPayload, repeatCount)
        return outcome.fold(
            onSuccess = { backend ->
                res.put("ok", true).put("backendUsed", backend).toString()
            },
            onFailure = { err ->
                res.put("ok", false).put("error", err.message ?: "IR transmit failed").toString()
            },
        )
    }
}
