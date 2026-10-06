package dev.orchid.voice

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.widget.*

class MainActivity : Activity() {
    private lateinit var settings: ConnectionSettings
    private lateinit var endpoint: EditText
    private lateinit var token: EditText
    private lateinit var status: TextView
    private lateinit var exchange: TextView
    private val handler = Handler(Looper.getMainLooper())
    private val refresh = object : Runnable { override fun run() { status.text = settings.status; exchange.text = settings.lastExchange; handler.postDelayed(this, 1000) } }
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        settings = ConnectionSettings(this)
        intent.getStringExtra("endpoint")?.let { settings.endpoint = it }
        intent.getStringExtra("token")?.let { settings.token = it }
        val root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(32, 48, 32, 32) }
        fun label(text: String) { root.addView(TextView(this).apply { this.text = text; setPadding(0, 16, 0, 8) }) }
        label("Orchid Voice")
        label("Keep Tailscale connected. Start here, then lock the phone and speak. Speech is transcribed locally on your laptop; this phone reads deliberate agent output.")
        label("Laptop endpoint")
        endpoint = EditText(this).apply { setSingleLine(); setText(settings.endpoint); hint = "ws://100.x.x.x:4818/phone" }; root.addView(endpoint)
        label("Access token")
        token = EditText(this).apply { setSingleLine(); inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD; setText(settings.token) }; root.addView(token)
        root.addView(Button(this).apply { text = "Start listening"; setOnClickListener { begin() } })
        root.addView(Button(this).apply { text = "Stop"; setOnClickListener { startService(Intent(this@MainActivity, ConversationService::class.java).setAction("STOP")) } })
        status = TextView(this).apply { setPadding(0, 20, 0, 20) }; root.addView(status)
        exchange = TextView(this); root.addView(exchange)
        label("Text input")
        val input = EditText(this).apply { hint = "Send a request" }; root.addView(input)
        root.addView(Button(this).apply { text = "Send"; setOnClickListener { if (input.text.isNotBlank()) { startService(Intent(this@MainActivity, ConversationService::class.java).setAction("TEXT").putExtra("text", input.text.toString())); input.setText("") } } })
        setContentView(ScrollView(this).apply { addView(root) })
    }
    private fun begin() {
        settings.endpoint = endpoint.text.toString().trim(); settings.token = token.text.toString().trim()
        try { settings.validate() } catch (error: Exception) { Toast.makeText(this, error.message, Toast.LENGTH_LONG).show(); return }
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) { requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 1); return }
        if (android.os.Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) { requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 2); return }
        startForegroundService(Intent(this, ConversationService::class.java).setAction("START"))
    }
    override fun onRequestPermissionsResult(code: Int, permissions: Array<out String>, results: IntArray) { super.onRequestPermissionsResult(code, permissions, results); if (results.isNotEmpty() && results[0] == PackageManager.PERMISSION_GRANTED) begin() }
    override fun onResume() { super.onResume(); handler.post(refresh) }
    override fun onPause() { handler.removeCallbacks(refresh); super.onPause() }
}
