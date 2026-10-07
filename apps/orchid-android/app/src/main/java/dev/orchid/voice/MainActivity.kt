package dev.orchid.voice

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.media.AudioManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.view.View
import android.widget.*
import okhttp3.Call
import org.json.JSONObject

class MainActivity : Activity() {
    private lateinit var settings: ConnectionSettings
    private lateinit var client: PhoneHistoryClient
    private lateinit var root: LinearLayout
    private lateinit var content: LinearLayout
    private lateinit var scroll: ScrollView
    private lateinit var status: TextView
    private lateinit var historyStatus: TextView
    private var exchange: TextView? = null
    private var volume: TextView? = null
    private var composer: EditText? = null
    private var draft = ""
    private var tab = 0
    private var selectedSession: String? = null
    private var selectedTitle = "Session"
    private var pending: Call? = null
    private var generation = 0
    private var nextBefore: String? = null
    private var sessions: JSONObject? = null
    private var exchanges = org.json.JSONArray()
    private val turns = linkedMapOf<String, JSONObject>()
    private val expanded = mutableSetOf<String>()
    private val collapsed = mutableSetOf<String>()
    private var rendered = ""
    private val handler = Handler(Looper.getMainLooper())
    private val refresh = object : Runnable {
        override fun run() {
            status.text = settings.status
            exchange?.text = settings.lastExchange.ifBlank { "Your conversation will appear here." }
            volume?.text = if (getSystemService(AudioManager::class.java).getStreamVolume(AudioManager.STREAM_MUSIC) == 0)
                "Media volume is muted" else "Speech follows your media volume"
            if (tab == 1 && pending == null) loadHistory()
            handler.postDelayed(this, 3000)
        }
    }

    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        volumeControlStream = AudioManager.STREAM_MUSIC
        settings = ConnectionSettings(this)
        client = PhoneHistoryClient(settings)
        intent.getStringExtra("endpoint")?.let { settings.endpoint = it }
        intent.getStringExtra("token")?.let { settings.token = it }
        tab = state?.getInt("tab") ?: 0
        selectedSession = state?.getString("session")
        selectedTitle = state?.getString("title") ?: "Session"
        draft = state?.getString("draft") ?: ""
        state?.getStringArrayList("expanded")?.let { expanded.addAll(it) }
        state?.getStringArrayList("collapsed")?.let { collapsed.addAll(it) }
        window.statusBarColor = PhoneTheme.canvas
        window.navigationBarColor = PhoneTheme.canvas
        root = PhoneTheme.column(this).apply { setBackgroundColor(PhoneTheme.canvas); fitsSystemWindows = true }
        val header = PhoneTheme.column(this).apply { setPadding(dp(20), dp(16), dp(20), dp(8)) }
        header.addView(PhoneTheme.text(this, "Orchid Voice", 24f, bold = true))
        status = PhoneTheme.text(this, settings.status, 13f, PhoneTheme.accent); header.addView(status)
        root.addView(header)
        scroll = ScrollView(this).apply { isFillViewport = true }
        content = PhoneTheme.column(this).apply { setPadding(dp(20), dp(8), dp(20), dp(20)) }
        scroll.addView(content); root.addView(scroll, LinearLayout.LayoutParams(-1, 0, 1f))
        val navigation = LinearLayout(this).apply { setPadding(dp(12), dp(8), dp(12), dp(12)); setBackgroundColor(PhoneTheme.surface) }
        listOf("Voice", "Sessions", "Settings").forEachIndexed { index, title ->
            navigation.addView(PhoneTheme.button(this, title) { switchTab(index) }, LinearLayout.LayoutParams(0, -2, 1f).apply { marginStart = dp(3); marginEnd = dp(3) })
        }
        root.addView(navigation); setContentView(root); showTab()
    }
    private fun dp(value: Int) = PhoneTheme.dp(this, value)
    private fun addText(parent: LinearLayout, text: String, size: Float = 15f, color: Int = PhoneTheme.primary, bold: Boolean = false) {
        parent.addView(PhoneTheme.text(this, text, size, color, bold).apply { setTextIsSelectable(true) })
    }
    private fun action(parent: LinearLayout, label: String, run: () -> Unit) {
        parent.addView(PhoneTheme.button(this, label, run), LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(8) })
    }
    private fun section(parent: LinearLayout, key: String, title: String, initiallyOpen: Boolean = false, build: (LinearLayout) -> Unit) {
        val card = PhoneTheme.card(this)
        val body = PhoneTheme.column(this)
        var open = key in expanded || (initiallyOpen && key !in collapsed)
        val toggle = PhoneTheme.button(this, "${if (open) "▾" else "▸"} $title") {}
        toggle.setTextColor(PhoneTheme.primary); toggle.gravity = android.view.Gravity.START or android.view.Gravity.CENTER_VERTICAL
        toggle.setOnClickListener {
            open = !open; body.visibility = if (open) View.VISIBLE else View.GONE
            toggle.text = "${if (open) "▾" else "▸"} $title"
            if (open) { expanded.add(key); collapsed.remove(key) } else { expanded.remove(key); collapsed.add(key) }
        }
        card.addView(toggle); build(body); body.visibility = if (open) View.VISIBLE else View.GONE; card.addView(body); parent.addView(card)
    }
    private fun switchTab(value: Int) {
        composer?.let { draft = it.text.toString() }
        pending?.cancel(); pending = null; generation++; tab = value; showTab()
        if (tab == 1) loadHistory()
    }
    private fun showTab() {
        content.removeAllViews(); exchange = null; volume = null; composer = null; rendered = ""
        val navigation = root.getChildAt(2) as LinearLayout
        for (index in 0 until navigation.childCount) {
            val button = navigation.getChildAt(index) as Button
            button.background = PhoneTheme.background(this, if (index == tab) PhoneTheme.selected else PhoneTheme.surface)
            button.isSelected = index == tab
        }
        when (tab) { 0 -> conversation(); 1 -> renderHistory(); 2 -> setup() }
    }
    private fun conversation() {
        addText(content, "Conversation", 20f, bold = true)
        addText(content, "Start listening, then lock your phone and speak. Address requests to Orchid.", color = PhoneTheme.secondary)
        val controls = PhoneTheme.card(this)
        val actions = LinearLayout(this)
        actions.addView(PhoneTheme.button(this, "Start listening") { begin() }, LinearLayout.LayoutParams(0, -2, 1f).apply { marginEnd = dp(6) })
        actions.addView(PhoneTheme.button(this, "Stop") { startService(Intent(this, ConversationService::class.java).setAction("STOP")) }, LinearLayout.LayoutParams(0, -2, 1f).apply { marginStart = dp(6) })
        controls.addView(actions)
        volume = PhoneTheme.text(this, "Speech follows your media volume", 13f, PhoneTheme.muted); controls.addView(volume)
        content.addView(controls)
        section(content, "exchange", "Latest exchange", true) {
            exchange = PhoneTheme.text(this, settings.lastExchange.ifBlank { "Your conversation will appear here." }).apply { setTextIsSelectable(true) }
            it.addView(exchange)
            action(it, "View session history") { switchTab(1) }
        }
        section(content, "text-input", "Send text", true) {
            val input = EditText(this).apply { hint = "Address a request to Orchid"; setText(draft); setTextColor(PhoneTheme.primary); setHintTextColor(PhoneTheme.muted); minLines = 2 }
            composer = input; it.addView(input)
            action(it, "Send") {
                if (input.text.isNotBlank()) {
                    startService(Intent(this, ConversationService::class.java).setAction("TEXT").putExtra("text", input.text.toString())); input.setText(""); draft = ""
                }
            }
        }
    }
    private fun setup() {
        addText(content, "Connection settings", 20f, bold = true)
        addText(content, "Keep Tailscale connected. Copy these details from Voice Sessions on your laptop.", color = PhoneTheme.secondary)
        val card = PhoneTheme.card(this)
        addText(card, "Laptop endpoint", 13f, PhoneTheme.secondary)
        val endpoint = EditText(this).apply { setSingleLine(); setText(settings.endpoint); hint = "ws://100.x.x.x:4818/phone"; inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI; setTextColor(PhoneTheme.primary) }; card.addView(endpoint)
        addText(card, "Access token", 13f, PhoneTheme.secondary)
        val token = EditText(this).apply { setSingleLine(); inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD; setText(settings.token); setTextColor(PhoneTheme.primary) }; card.addView(token)
        action(card, "Save connection") {
            settings.endpoint = endpoint.text.toString().trim(); settings.token = token.text.toString().trim()
            try { settings.validate(); Toast.makeText(this, "Connection saved. Restart listening to apply it.", Toast.LENGTH_SHORT).show() }
            catch (error: Exception) { Toast.makeText(this, error.message, Toast.LENGTH_LONG).show() }
        }
        content.addView(card)
        section(content, "privacy", "Audio and history", true) {
            addText(it, "Audio is transcribed on your laptop. Deliberate agent output is spoken on your phone at media volume. Browsing Sessions does not start your microphone.", color = PhoneTheme.secondary)
            addText(it, "Session history comes from the laptop and updates while this tab is open.", color = PhoneTheme.secondary)
        }
    }
    private fun loadHistory(before: String? = null) {
        if (pending != null) return
        val epoch = generation
        val path = selectedSession?.let { "/sessions/$it" + (before?.let { cursor -> "?before=$cursor" } ?: "") } ?: "/sessions"
        pending = client.get(path) { value, error ->
            if (epoch != generation) return@get
            pending = null
            if (error != null) { if (::historyStatus.isInitialized) { historyStatus.text = error; historyStatus.setTextColor(PhoneTheme.danger) }; return@get }
            if (value == null) return@get
            if (selectedSession == null) sessions = value else {
                selectedTitle = value.optString("title", selectedTitle)
                exchanges = value.optJSONArray("exchanges") ?: org.json.JSONArray()
                if (before != null || turns.isEmpty()) nextBefore = value.optString("nextBefore").takeUnless { it == "null" || it.isBlank() }
                val entries = value.optJSONArray("turns")
                if (entries != null) for (index in 0 until entries.length()) { val turn = entries.getJSONObject(index); turns[turn.getString("id")] = turn }
            }
            val signature = if (selectedSession == null) sessions.toString() else turns.values.joinToString() + nextBefore + exchanges
            if (signature != rendered) { val y = scroll.scrollY; renderHistory(); rendered = signature; scroll.post { scroll.scrollTo(0, y) } }
            historyStatus.text = "Updated · read only"; historyStatus.setTextColor(PhoneTheme.muted)
        }
    }
    private fun renderHistory() {
        content.removeAllViews()
        if (selectedSession == null) {
            addText(content, "Sessions", 20f, bold = true)
            addText(content, "Follow the conversation and the agents working behind it.", color = PhoneTheme.secondary)
            val entries = sessions?.optJSONArray("sessions")
            for ((role, title) in listOf("conversation" to "Main interaction", "worker" to "Working sessions")) {
                section(content, "group-$role", title, true) { body ->
                    var count = 0
                    if (entries != null) for (index in 0 until entries.length()) {
                        val entry = entries.getJSONObject(index)
                        if (entry.optString("role") != role) continue
                        count++
                        val name = entry.optString("title", "Session")
                        action(body, "$name · ${entry.optString("status").takeUnless { it == "null" } ?: "Ready"}") {
                            selectedSession = entry.getString("id"); selectedTitle = name; turns.clear(); nextBefore = null; exchanges = org.json.JSONArray()
                            generation++; pending?.cancel(); pending = null; rendered = ""; renderHistory(); loadHistory()
                        }
                        if (role == "worker") addText(body, entry.optString("task"), 13f, PhoneTheme.secondary)
                    }
                    if (count == 0) addText(body, if (role == "worker") "No delegated work yet." else "Connect to your laptop to load the conversation.", color = PhoneTheme.muted)
                }
            }
        } else {
            action(content, "‹ All sessions") { selectedSession = null; generation++; pending?.cancel(); pending = null; rendered = ""; renderHistory(); loadHistory() }
            addText(content, selectedTitle, 20f, bold = true)
            if (exchanges.length() > 0) section(content, "speech-$selectedSession", "Spoken exchanges (${exchanges.length()})") { body ->
                for (index in 0 until exchanges.length()) {
                    val entry = exchanges.getJSONObject(index)
                    addText(body, entry.optString("speaker"), 13f, PhoneTheme.accent, true)
                    addText(body, entry.optString("text"))
                }
            }
            if (nextBefore != null) action(content, "Load earlier turns") { loadHistory(nextBefore) }
            if (turns.isEmpty()) addText(content, "No turns yet. This view updates as the session runs.", color = PhoneTheme.muted)
            for (turn in turns.values.sortedBy { it.optString("createdAt") }) {
                val id = turn.getString("id")
                val card = PhoneTheme.card(this)
                addText(card, "${turn.optString("status")} · ${turn.optString("createdAt").replace('T', ' ').take(19)}", 13f, PhoneTheme.accent)
                section(card, "$id-request", if (turn.optString("provenance") == "user" || turn.optString("inputKind") == "speech") "Your input" else "Input / context", true) { addText(it, turn.optString("request")) }
                val reply = turn.optString("reply").takeUnless { it == "null" || it.isBlank() }
                if (reply != null) section(card, "$id-reply", "Agent response", true) { addText(it, reply) }
                val error = turn.optString("error").takeUnless { it == "null" || it.isBlank() }
                if (error != null) addText(card, error, color = PhoneTheme.danger)
                val events = turn.optJSONArray("events")
                if (events != null && events.length() > 0) section(card, "$id-activity", "Activity (${events.length()})") { body ->
                    for (index in 0 until events.length()) {
                        val event = events.getJSONObject(index)
                        val tool = event.optJSONObject("tool")
                        val label = tool?.let { "${it.optString("kind")} · ${it.optString("phase")}" } ?: event.optString("kind").replace('_', ' ')
                        addText(body, label, 12f, PhoneTheme.accent, true)
                        addText(body, event.optString("text"), 14f, PhoneTheme.secondary)
                    }
                }
                content.addView(card)
            }
        }
        historyStatus = PhoneTheme.text(this, "Loading from laptop…", 13f, PhoneTheme.muted); content.addView(historyStatus)
        action(content, "Refresh") { loadHistory() }
    }
    private fun begin() {
        try { settings.validate() } catch (error: Exception) { Toast.makeText(this, error.message, Toast.LENGTH_LONG).show(); switchTab(2); return }
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) { requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 1); return }
        if (android.os.Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) { requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 2); return }
        startForegroundService(Intent(this, ConversationService::class.java).setAction("START"))
    }
    override fun onRequestPermissionsResult(code: Int, permissions: Array<out String>, results: IntArray) { super.onRequestPermissionsResult(code, permissions, results); if (results.isNotEmpty() && results[0] == PackageManager.PERMISSION_GRANTED) begin() }
    override fun onResume() { super.onResume(); handler.post(refresh) }
    override fun onPause() { handler.removeCallbacks(refresh); pending?.cancel(); pending = null; generation++; super.onPause() }
    override fun onDestroy() { client.close(); super.onDestroy() }
    override fun onSaveInstanceState(state: Bundle) { super.onSaveInstanceState(state); state.putInt("tab", tab); state.putString("session", selectedSession); state.putString("title", selectedTitle); state.putString("draft", composer?.text?.toString() ?: draft); state.putStringArrayList("expanded", ArrayList(expanded)); state.putStringArrayList("collapsed", ArrayList(collapsed)) }
    @Deprecated("Activity back navigation") override fun onBackPressed() { if (tab == 1 && selectedSession != null) { selectedSession = null; switchTab(1) } else super.onBackPressed() }
}
