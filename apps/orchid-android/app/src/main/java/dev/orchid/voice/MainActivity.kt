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
    private var live: TextView? = null
    private var liveDetails: TextView? = null
    private var levelMeter: ProgressBar? = null
    private var liveCall: Call? = null
    private var liveState: JSONObject? = null
    private var liveError: String? = null
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
    private var inputDecisions = org.json.JSONArray()
    private var inputBuffer = JSONObject()
    private val turns = linkedMapOf<String, JSONObject>()
    private val expanded = mutableSetOf<String>()
    private val collapsed = mutableSetOf<String>()
    private var rendered = ""
    private val handler = Handler(Looper.getMainLooper())
    private val refresh = object : Runnable {
        override fun run() {
            val capture = runCatching { JSONObject(settings.captureStatus) }.getOrDefault(JSONObject())
            val fresh = runCatching { java.time.Instant.parse(capture.optString("updatedAt")).toEpochMilli() > System.currentTimeMillis() - 5000 }.getOrDefault(false)
            status.text = if (capture.optBoolean("recording") && fresh) settings.status else capture.optString("error").takeUnless { it == "null" || it.isBlank() } ?: "Stopped"
            exchange?.text = settings.lastExchange.ifBlank { "Your conversation will appear here." }
            volume?.text = if (getSystemService(AudioManager::class.java).getStreamVolume(AudioManager.STREAM_MUSIC) == 0)
                "Media volume is muted" else "Speech follows your media volume"
            if (tab == 0) { renderLive(); loadLive() }
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
        content.removeAllViews(); exchange = null; volume = null; composer = null; live = null; liveDetails = null; levelMeter = null; rendered = ""
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
        val diagnostics = PhoneTheme.card(this)
        addText(diagnostics, "Live status", 18f, bold = true)
        val microphones = RadioGroup(this).apply { orientation = RadioGroup.HORIZONTAL }
        listOf("phone" to "Phone", "headset" to "External microphone").forEach { (value, label) ->
            microphones.addView(RadioButton(this).apply {
                id = View.generateViewId(); text = label; setTextColor(PhoneTheme.primary); minHeight = dp(48)
                isChecked = settings.microphone == value
                setOnCheckedChangeListener { _, checked -> if (checked && settings.microphone != value) changeMicrophone(value) }
            })
        }
        diagnostics.addView(microphones)
        levelMeter = ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal).apply { max = 100 }
        diagnostics.addView(levelMeter, LinearLayout.LayoutParams(-1, dp(12)))
        live = PhoneTheme.text(this, "Checking microphone and laptop…", 14f).apply { setTextIsSelectable(true) }
        diagnostics.addView(live); content.addView(diagnostics)
        section(content, "pipeline-status", "Input and reasoning details") {
            liveDetails = PhoneTheme.text(this, "Waiting for laptop status…", 14f).apply { setTextIsSelectable(true) }
            it.addView(liveDetails)
        }
        renderLive(); loadLive()
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
        section(content, "assistant", "Assistant shortcuts", true) { body ->
            val role = getSystemService(android.app.role.RoleManager::class.java)
            addText(body, if (role.isRoleHeld(android.app.role.RoleManager.ROLE_ASSISTANT)) "Orchid is your phone assistant." else "Choose Orchid as your phone assistant to use the assistant gesture and connected device buttons.", color = PhoneTheme.secondary)
            action(body, "Choose phone assistant") {
                if (role.isRoleAvailable(android.app.role.RoleManager.ROLE_ASSISTANT) && !role.isRoleHeld(android.app.role.RoleManager.ROLE_ASSISTANT)) startActivityForResult(role.createRequestRoleIntent(android.app.role.RoleManager.ROLE_ASSISTANT), 3)
                else startActivity(Intent(android.provider.Settings.ACTION_MANAGE_DEFAULT_APPS_SETTINGS))
            }
            addText(body, "Garmin and headset buttons depend on the device's assistant support. Your microphone setting still applies. A custom wake phrase is not enabled.", color = PhoneTheme.secondary)
        }
        section(content, "microphone", "Microphone", true) { body ->
            val choices = RadioGroup(this)
            listOf("phone" to "Phone microphone · preserve playback quality", "headset" to "Headset microphone · Bluetooth call quality").forEach { (value, label) ->
                choices.addView(RadioButton(this).apply {
                    id = View.generateViewId(); text = label; setTextColor(PhoneTheme.primary); minHeight = dp(48)
                    isChecked = settings.microphone == value
                    setOnCheckedChangeListener { _, checked -> if (checked && settings.microphone != value) changeMicrophone(value) }
                })
            }
            body.addView(choices)
            addText(body, "Applies immediately while listening. Classic Bluetooth headsets switch to lower-quality call audio when their microphone is active. The Voice tab shows the actual route.", color = PhoneTheme.secondary)
        }
        section(content, "background-audio", "Other audio during speech", true) { body ->
            val choices = RadioGroup(this)
            listOf("lower" to "Lower other audio", "pause" to "Pause other audio", "off" to "Keep other audio unchanged").forEach { (value, label) ->
                val choice = RadioButton(this).apply {
                    id = View.generateViewId(); text = label; setTextColor(PhoneTheme.primary); minHeight = dp(48)
                    isChecked = settings.backgroundAudio == value
                    setOnCheckedChangeListener { _, checked -> if (checked) settings.backgroundAudio = value }
                }
                choices.addView(choice)
            }
            body.addView(choices)
            addText(body, "Applies while Orchid speaks, then restores the previous app. Android controls how much other audio is lowered; some spoken content pauses instead.", color = PhoneTheme.secondary)
        }
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
                inputDecisions = value.optJSONArray("inputDecisions") ?: org.json.JSONArray()
                inputBuffer = value.optJSONObject("inputBuffer") ?: JSONObject()
                if (before != null || turns.isEmpty()) nextBefore = value.optString("nextBefore").takeUnless { it == "null" || it.isBlank() }
                val entries = value.optJSONArray("turns")
                if (entries != null) for (index in 0 until entries.length()) { val turn = entries.getJSONObject(index); turns[turn.getString("id")] = turn }
            }
            val signature = if (selectedSession == null) sessions.toString() else turns.values.joinToString() + nextBefore + exchanges + inputDecisions + inputBuffer
            if (signature != rendered) { val y = scroll.scrollY; renderHistory(); rendered = signature; scroll.post { scroll.scrollTo(0, y) } }
            historyStatus.text = "Updated · read only"; historyStatus.setTextColor(PhoneTheme.muted)
        }
    }
    private fun changeMicrophone(value: String) {
        settings.microphone = value
        val capture = runCatching { JSONObject(settings.captureStatus) }.getOrDefault(JSONObject())
        if (capture.optBoolean("recording")) startService(Intent(this, ConversationService::class.java).setAction("MICROPHONE").putExtra("microphone", value))
        renderLive()
    }
    private fun loadLive() {
        if (liveCall != null || settings.endpoint.isBlank()) return
        liveCall = client.get("/status") { value, error ->
            liveCall = null; liveState = value; liveError = error; renderLive()
        }
    }
    private fun renderLive() {
        if (live == null) return
        val capture = runCatching { JSONObject(settings.captureStatus) }.getOrDefault(JSONObject())
        val fresh = runCatching { java.time.Instant.parse(capture.optString("updatedAt")).toEpochMilli() > System.currentTimeMillis() - 5000 }.getOrDefault(false)
        val recording = capture.optBoolean("recording") && fresh
        val lines = mutableListOf<String>()
        val details = mutableListOf<String>()
        lines += if (!recording) "Microphone: stopped · tap Start listening" else "Microphone: ${capture.optString("routedMicrophone", "Checking route")}${if (capture.optBoolean("silenced")) " · blocked by Android" else " · recording"}"
        if (recording && settings.microphone == "headset" && !capture.optBoolean("external")) lines += "External microphone requested; Android is using the phone. Check the headset's call audio connection."
        capture.optString("error").takeUnless { it == "null" || it.isBlank() }?.let { lines += "Capture error: $it" }
        val level = if (recording) capture.optDouble("level", 0.0) else 0.0
        levelMeter?.progress = (kotlin.math.sqrt(level.coerceIn(0.0, 1.0)) * 100).toInt()
        lines += "Input signal: ${String.format(java.util.Locale.ROOT, "%.1f", level * 100)}% · ${if (recording && capture.optBoolean("speechDetected")) "speech detected" else "no speech detected"}"
        lines += "Audio segments: ${capture.optLong("segmentsCaptured")} recorded · ${capture.optLong("segmentsSent")} sent"
        if (liveError != null) lines += "Laptop status: $liveError"
        liveState?.let { state ->
            val audio = state.optJSONObject("audio") ?: JSONObject()
            lines += "Laptop: ${audio.optLong("received")} segments received · ${audio.optLong("transcribed")} transcribed · ${state.optInt("pendingAudio")} queued"
            val input = state.optJSONObject("input") ?: JSONObject()
            lines += "Input: ${input.optString("title")}"
            details += input.optString("detail")
            if (input.optString("title") == "Waiting for you to finish") lines += "Indicate when you're ready to send."
            input.optJSONObject("last")?.let { last ->
                details += "Last input: ${last.optString("text")}"
                details += "JEV: ${if (last.optBoolean("filterPending")) "evaluating" else last.optString("relevance")} · ${last.optString("completion").takeUnless { it == "null" } ?: "not checked"}"
            }
            val stages = state.optJSONArray("stages")
            if (stages != null) for (index in 0 until stages.length()) {
                val stage = stages.getJSONObject(index)
                val label = "${stage.optString("role").replaceFirstChar { it.uppercase() }}: ${stage.optString("status").takeUnless { it == "null" } ?: "not started"}${if (stage.optInt("queued") > 0) " · ${stage.optInt("queued")} queued" else ""}"
                details += label
                if (stage.optString("status") !in listOf("null", "completed") || stage.optInt("queued") > 0) lines += label
                stage.optString("error").takeUnless { it == "null" || it.isBlank() }?.let { lines += it }
            }
            state.optString("error").takeUnless { it == "null" || it.isBlank() }?.let { lines += "Processing error: $it" }
        }
        live?.text = lines.joinToString("\n\n")
        liveDetails?.text = details.joinToString("\n\n")
    }
    private fun renderHistory() {
        content.removeAllViews()
        if (selectedSession == null) {
            addText(content, "Sessions", 20f, bold = true)
            addText(content, "Follow the conversation and the agents working behind it.", color = PhoneTheme.secondary)
            val entries = sessions?.optJSONArray("sessions")
            for ((role, title) in listOf("conversation" to "Main interaction", "reasoning" to "Reasoning sessions", "worker" to "Working sessions")) {
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
            val held = inputBuffer.optJSONArray("blocks")
            if (held != null && held.length() > 0) section(content, "held-$selectedSession", "Accepted input held (${held.length()})", true) { body ->
                for (index in 0 until held.length()) addText(body, held.getJSONObject(index).optString("text"))
                addText(body, if (inputBuffer.optBoolean("complete")) "Ready · waiting for speech and queued processing" else "Waiting for the input to be complete", 13f, PhoneTheme.muted)
            }
            if (inputDecisions.length() > 0) section(content, "decisions-$selectedSession", "Recent speech decisions") { body ->
                for (index in maxOf(0, inputDecisions.length() - 10) until inputDecisions.length()) {
                    val entry = inputDecisions.getJSONObject(index)
                    val relevance = entry.optString("relevance").takeUnless { it == "null" || it.isBlank() } ?: "Earlier input"
                    val completion = entry.optString("completion").takeUnless { it == "null" || it.isBlank() }
                    addText(body, if (entry.optBoolean("filterPending")) "Evaluating" else relevance + (completion?.let { " · $it" } ?: ""), 13f, PhoneTheme.accent, true)
                    addText(body, entry.optString("text"))
                }
            }
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
    override fun onActivityResult(code: Int, result: Int, data: Intent?) { super.onActivityResult(code, result, data); if (code == 3 && tab == 2) showTab() }
    override fun onResume() { super.onResume(); handler.post(refresh) }
    override fun onPause() { handler.removeCallbacks(refresh); pending?.cancel(); pending = null; liveCall?.cancel(); liveCall = null; generation++; super.onPause() }
    override fun onDestroy() { client.close(); super.onDestroy() }
    override fun onSaveInstanceState(state: Bundle) { super.onSaveInstanceState(state); state.putInt("tab", tab); state.putString("session", selectedSession); state.putString("title", selectedTitle); state.putString("draft", composer?.text?.toString() ?: draft); state.putStringArrayList("expanded", ArrayList(expanded)); state.putStringArrayList("collapsed", ArrayList(collapsed)) }
    @Deprecated("Activity back navigation") override fun onBackPressed() { if (tab == 1 && selectedSession != null) { selectedSession = null; switchTab(1) } else super.onBackPressed() }
}
