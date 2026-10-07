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
import android.widget.TextView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.ProgressBar

/** The locked-screen entry point exposes controls without revealing history or credentials. */
class AssistantActivity : Activity() {
    private lateinit var settings: ConnectionSettings
    private lateinit var status: TextView
    private lateinit var captureStatus: TextView
    private lateinit var levelMeter: ProgressBar
    private var activationPending = true
    private val handler = Handler(Looper.getMainLooper())
    private val refresh = object : Runnable {
        override fun run() {
            status.text = settings.status
            val capture = runCatching { org.json.JSONObject(settings.captureStatus) }.getOrDefault(org.json.JSONObject())
            val fresh = runCatching { java.time.Instant.parse(capture.optString("updatedAt")).toEpochMilli() > System.currentTimeMillis() - 5000 }.getOrDefault(false)
            val recording = capture.optBoolean("recording") && fresh
            levelMeter.progress = if (recording) (kotlin.math.sqrt(capture.optDouble("level", 0.0).coerceIn(0.0, 1.0)) * 100).toInt() else 0
            captureStatus.text = if (recording)
                "Microphone: ${capture.optString("routedMicrophone")}\nInput signal: ${String.format(java.util.Locale.ROOT, "%.1f", capture.optDouble("level") * 100)}%${if (capture.optBoolean("silenced")) " · blocked by Android" else ""}\n${if (capture.optBoolean("speechDetected")) "Speech detected" else "No speech detected"} · ${capture.optLong("segmentsSent")} segments sent\nListening timeout in ${capture.optLong("idleRemainingSeconds")} seconds"
                else "Microphone stopped · tap Start listening"
            handler.postDelayed(this, 1000)
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
        root.addView(PhoneTheme.text(this, "Orchid", 28f, bold = true))
        status = PhoneTheme.text(this, settings.status, 16f, PhoneTheme.accent)
        root.addView(status)
        captureStatus = PhoneTheme.text(this, "Checking microphone…", 14f, PhoneTheme.secondary)
        root.addView(captureStatus)
        levelMeter = ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal).apply { max = 100 }
        root.addView(levelMeter, LinearLayout.LayoutParams(-1, PhoneTheme.dp(this, 12)))
        root.addView(PhoneTheme.text(this, "Address your request to Orchid. Listening stops after ${settings.listeningTimeoutSeconds} seconds without relevant input. Start listening to resume.", color = PhoneTheme.secondary))
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
        frame.addView(ScrollView(this).apply { addView(root) })
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
            status.text = "Open Orchid and save your laptop connection first."
            handler.removeCallbacks(refresh)
            return
        }
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            status.text = "Unlock your phone and allow Orchid to use the microphone."
            handler.removeCallbacks(refresh)
            if (!getSystemService(KeyguardManager::class.java).isKeyguardLocked) requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 1)
            return
        }
        try {
            startForegroundService(Intent(this, ConversationService::class.java).setAction("START"))
            handler.removeCallbacks(refresh); handler.postDelayed(refresh, 250)
        } catch (_: Exception) {
            status.text = "Unable to start listening. Unlock your phone and try again."
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
