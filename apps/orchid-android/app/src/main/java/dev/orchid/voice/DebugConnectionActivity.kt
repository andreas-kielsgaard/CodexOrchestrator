package dev.orchid.voice

import android.Manifest
import android.app.Activity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.view.WindowManager
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

/** Manual, foreground-only bootstrap. No receivers, boot hooks, or external app dependencies. */
class DebugConnectionActivity : Activity() {
    private lateinit var connection: DebugConnection
    private lateinit var status: TextView
    private lateinit var start: Button
    private lateinit var copy: Button
    private var endpoint: String? = null

    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        window.statusBarColor = PhoneTheme.canvas
        window.navigationBarColor = PhoneTheme.canvas
        val body = PhoneTheme.column(this).apply {
            setPadding(PhoneTheme.dp(context, 20), PhoneTheme.dp(context, 32), PhoneTheme.dp(context, 20), PhoneTheme.dp(context, 20))
            setBackgroundColor(PhoneTheme.canvas)
            fitsSystemWindows = true
        }
        fun text(value: String, size: Float = 15f): TextView = PhoneTheme.text(this, value, size).also {
            it.setTextIsSelectable(true); body.addView(it)
        }
        fun button(label: String, action: () -> Unit): Button = PhoneTheme.button(this, label, action).also {
            body.addView(it, LinearLayout.LayoutParams(-1, -2).apply { topMargin = PhoneTheme.dp(this@DebugConnectionActivity, 12) })
        }
        button("Back to settings") { finish() }
        text("Debugging connection", 24f)
        text("Start phone debugging manually over Tailscale. The mobile-data startup method is experimental. Keep this screen open and Tailscale connected. Turn Mobile Hotspot off before starting; Orchid can create a temporary hotspot without joining another Wi-Fi network.")
        status = text("Ready", 17f)
        start = button("Start debugging connection") { begin() }
        button("Cancel / release temporary hotspot") { connection.cancel(); window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON) }
        copy = button("Copy endpoint") {
            endpoint?.let { getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("Debugging endpoint", it)) }
        }.apply { isEnabled = false }
        button("Android developer options") { startActivity(Intent(Settings.ACTION_APPLICATION_DEVELOPMENT_SETTINGS)) }
        text("One-time setup", 19f)
        text("While an ADB connection is available, grant Orchid permission from your computer. This does not grant root. The permission normally remains after restarting the phone.")
        val command = "adb shell pm grant $packageName android.permission.WRITE_SECURE_SETTINGS"
        text(command, 13f)
        button("Copy setup command") { getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("Orchid setup", command)) }
        text("Once an endpoint appears, connect from the computer with adb connect <endpoint>. Accept or pair the computer in Android developer options if needed. After connecting, adb tcpip 5555 can keep access available over mobile data when this screen closes.", 13f)
        setContentView(ScrollView(this).apply { addView(body) })
        connection = DebugConnection(this) { message, found, busy ->
            status.text = message
            endpoint = found
            copy.isEnabled = found != null
            start.isEnabled = !busy
            if (!busy && found == null) window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        }
    }

    private fun begin() {
        if (checkSelfPermission(Manifest.permission.WRITE_SECURE_SETTINGS) != PackageManager.PERMISSION_GRANTED) {
            status.text = "Setup needed: grant Orchid the permission shown below while ADB is available."
            return
        }
        val permission = if (Build.VERSION.SDK_INT >= 33) Manifest.permission.NEARBY_WIFI_DEVICES else Manifest.permission.ACCESS_FINE_LOCATION
        if (checkSelfPermission(permission) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(if (Build.VERSION.SDK_INT >= 33) arrayOf(permission) else arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION), 10)
            return
        }
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        connection.start()
    }

    override fun onRequestPermissionsResult(code: Int, permissions: Array<out String>, results: IntArray) {
        super.onRequestPermissionsResult(code, permissions, results)
        if (code == 10 && results.firstOrNull() == PackageManager.PERMISSION_GRANTED) begin()
        else if (code == 10) status.text = "Nearby Wi-Fi permission is needed to create the temporary hotspot."
    }

    override fun onStop() { connection.cancel(); window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON); super.onStop() }
    override fun onDestroy() { connection.close(); super.onDestroy() }
}
