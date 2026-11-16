package com.panasonic.diga.engineering

import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import com.panasonic.diga.engineering.ir.AndroidIrBridge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webView.addJavascriptInterface(AndroidPicoBridge(this, webView), "__PANADIGA_ANDROID_BRIDGE__")
    webView.addJavascriptInterface(AndroidIrBridge(this), "__PANADIGA_IR_BRIDGE__")
  }
}
