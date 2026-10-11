package dev.orchid.voice

import android.app.Instrumentation
import android.content.Intent
import android.media.AudioDeviceInfo
import android.os.Bundle
import android.os.SystemClock
import android.view.View
import android.view.ViewGroup
import android.view.accessibility.AccessibilityNodeInfo

object AudioDevicesProbe {
    fun run(test: Instrumentation, result: Bundle) {
        val activity = test.startActivitySync(Intent(test.targetContext, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) as MainActivity
        test.runOnMainSync { MainActivity::class.java.getDeclaredMethod("switchTab", Int::class.javaPrimitiveType).apply { isAccessible = true }.invoke(activity, 3) }
        val settings = ConnectionSettings(activity)
        val devices = AudioDevices(activity)
        fun find(view: View, label: String): View? {
            if (view.contentDescription?.toString() == label) return view
            if (view is ViewGroup) for (index in 0 until view.childCount) find(view.getChildAt(index), label)?.let { return it }
            return null
        }
        fun choose(control: String, label: String) {
            val previousMic = settings.microphone; val previousOutput = settings.audioOutput
            test.runOnMainSync { check(find(activity.window.decorView, control)?.performClick() == true) }
            test.waitForIdleSync(); SystemClock.sleep(300)
            check(settings.microphone == previousMic && settings.audioOutput == previousOutput) { "Opening selector changed settings" }
            val root = test.uiAutomation.rootInActiveWindow ?: error("No selector window")
            val choice = root.findAccessibilityNodeInfosByText(label).firstOrNull { it.text?.toString() == label } ?: error("Selector did not show $label")
            check(choice.performAction(AccessibilityNodeInfo.ACTION_CLICK)) { "Choice could not be selected" }
            test.waitForIdleSync(); SystemClock.sleep(300)
        }
        val originalOutput = settings.audioOutput
        choose("Microphone", "Phone microphone")
        check(settings.microphone == "phone" && settings.audioOutput == originalOutput)
        val speaker = devices.outputs().first { it.type == AudioDeviceInfo.TYPE_BUILTIN_SPEAKER }
        choose("Audio output", AudioDevices.label(speaker))
        check(settings.microphone == "phone" && settings.audioOutput == AudioDevices.key(speaker))
        val bluetooth = devices.outputs().first { it.type == AudioDeviceInfo.TYPE_BLUETOOTH_A2DP }
        choose("Audio output", AudioDevices.label(bluetooth))
        check(settings.microphone == "phone" && settings.audioOutput == AudioDevices.key(bluetooth))
        result.putString("selectors", "Phone microphone and speaker/Bluetooth selected independently through visible pickers; opening preserves preferences")
    }
}
