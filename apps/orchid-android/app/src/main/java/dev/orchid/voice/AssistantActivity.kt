package dev.orchid.voice

import android.Manifest
import android.app.Activity
import android.app.KeyguardManager
import android.content.Intent
import android.content.pm.PackageManager
import android.media.AudioManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.widget.LinearLayout
import android.widget.Toast

/** Assistant activation controls and recent unfiltered transcripts. */
class AssistantActivity : Activity() {
    private lateinit var settings: ConnectionSettings
    private lateinit var inputScroll: ListeningTranscriptView
    private var activationPending = true
    private val handler = Handler(Looper.getMainLooper())
    private val refresh = object : Runnable {
        override fun run() {
            inputScroll.refresh()
            handler.postDelayed(this, 250)
        }
    }
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        settings = ConnectionSettings(this)
        activationPending = state?.getBoolean("activationPending") ?: true
        volumeControlStream = AudioManager.STREAM_MUSIC
        val frame = PhoneTheme.column(this).apply {
            setBackgroundColor(PhoneTheme.canvas)
            fitsSystemWindows = true
        }
        val root = PhoneTheme.column(this).apply {
            setPadding(PhoneTheme.dp(this@AssistantActivity, 24), PhoneTheme.dp(this@AssistantActivity, 24), PhoneTheme.dp(this@AssistantActivity, 24), PhoneTheme.dp(this@AssistantActivity, 24))
        }
        fun action(label: String, run: () -> Unit) {
            root.addView(PhoneTheme.button(this, label, run), LinearLayout.LayoutParams(-1, -2).apply { topMargin = PhoneTheme.dp(this@AssistantActivity, 8) })
        }
        action("Start listening") { begin() }
        action("Stop listening") {
            startService(Intent(this, ConversationService::class.java).setAction("STOP"))
            activationPending = false
            finish()
        }
        action("Open Orchid") { openApp() }
        action("Dismiss") { finish() }
        inputScroll = ListeningTranscriptView(this, settings)
        root.addView(inputScroll, LinearLayout.LayoutParams(-1, 0, 1f).apply { topMargin = PhoneTheme.dp(this@AssistantActivity, 16) })
        frame.addView(root, LinearLayout.LayoutParams(-1, -1))
        setContentView(frame)
    }
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        activationPending = true
    }
    override fun onResume() {
        super.onResume()
        handler.post(refresh)
        if (activationPending) { activationPending = false; begin() }
    }
    override fun onPause() { handler.removeCallbacks(refresh); super.onPause() }
    private fun begin() {
        try { settings.validate() } catch (_: Exception) {
            Toast.makeText(this, "Open Orchid and save your laptop connection first.", Toast.LENGTH_LONG).show()
            handler.removeCallbacks(refresh)
            return
        }
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            Toast.makeText(this, "Unlock your phone and allow Orchid to use the microphone.", Toast.LENGTH_LONG).show()
            handler.removeCallbacks(refresh)
            if (!getSystemService(KeyguardManager::class.java).isKeyguardLocked) requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 1)
            return
        }
        try {
            startForegroundService(Intent(this, ConversationService::class.java).setAction("START"))
            handler.removeCallbacks(refresh); handler.postDelayed(refresh, 250)
        } catch (_: Exception) {
            Toast.makeText(this, "Unable to start listening. Unlock your phone and try again.", Toast.LENGTH_LONG).show()
            handler.removeCallbacks(refresh)
        }
    }
    private fun openApp() {
        val keyguard = getSystemService(KeyguardManager::class.java)
        if (keyguard.isKeyguardLocked) {
            keyguard.requestDismissKeyguard(this, object : KeyguardManager.KeyguardDismissCallback() {
                override fun onDismissSucceeded() { openApp() }
            })
            return
        }
        startActivity(Intent(this, MainActivity::class.java))
        finish()
    }
    override fun onRequestPermissionsResult(code: Int, permissions: Array<out String>, results: IntArray) {
        super.onRequestPermissionsResult(code, permissions, results)
        if (code == 1 && results.firstOrNull() == PackageManager.PERMISSION_GRANTED) begin()
    }
    override fun onSaveInstanceState(state: Bundle) { super.onSaveInstanceState(state); state.putBoolean("activationPending", activationPending) }
}
