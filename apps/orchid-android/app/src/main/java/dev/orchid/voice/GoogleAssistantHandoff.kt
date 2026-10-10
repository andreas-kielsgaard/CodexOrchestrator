package dev.orchid.voice

import android.accessibilityservice.AccessibilityService
import android.content.Intent
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityWindowInfo

/** Opens Orchid when Google's assistant overlay appears. */
class GoogleAssistantHandoff : AccessibilityService() {
    private val handler = Handler(Looper.getMainLooper())
    private var lastHandoff = -3000L
    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (!ConnectionSettings(this).googleHandoff || SystemClock.elapsedRealtime() - lastHandoff < 3000) return
        val assistant = windows.any { window ->
            if (window.type !in listOf(AccessibilityWindowInfo.TYPE_SYSTEM, AccessibilityWindowInfo.TYPE_APPLICATION) || !window.isFocused) return@any false
            val root = window.root ?: return@any false
            root.packageName?.toString() == GOOGLE && MARKERS.any { root.findAccessibilityNodeInfosByViewId("$GOOGLE:id/$it").isNotEmpty() }
        }
        if (!assistant) return
        lastHandoff = SystemClock.elapsedRealtime()
        if (!performGlobalAction(GLOBAL_ACTION_BACK)) return
        handler.postDelayed({
            if (ConnectionSettings(this).googleHandoff) startActivity(Intent(Intent.ACTION_ASSIST)
                .setClass(this, AssistantActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        }, 200)
    }
    override fun onInterrupt() { handler.removeCallbacksAndMessages(null) }
    override fun onDestroy() { handler.removeCallbacksAndMessages(null); super.onDestroy() }
    companion object {
        private const val GOOGLE = "com.google.android.googlequicksearchbox"
        private val MARKERS = listOf("assistant_zero_state_container", "assistant_robin_zero_state_content_sheet", "assistant_robin_floaty_root_container")
    }
}
