package org.coachsplaybook.companion

import android.content.pm.ActivityInfo
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    // window.CoachScreen.immersiveLandscape(true|false), used by the full-screen tactics board.
    webView.addJavascriptInterface(ScreenBridge(), "CoachScreen")
  }

  inner class ScreenBridge {
    @JavascriptInterface
    fun immersiveLandscape(on: Boolean) {
      runOnUiThread {
        requestedOrientation =
          if (on) ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
          else ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
        val bars = WindowCompat.getInsetsController(window, window.decorView)
        if (on) {
          bars.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
          bars.hide(WindowInsetsCompat.Type.systemBars())
        } else {
          bars.show(WindowInsetsCompat.Type.systemBars())
        }
      }
    }
  }
}
