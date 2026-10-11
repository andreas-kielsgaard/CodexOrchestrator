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
import android.view.Gravity
import android.widget.*
import androidx.drawerlayout.widget.DrawerLayout
import androidx.core.view.GravityCompat
import okhttp3.Call
import org.json.JSONObject

class MainActivity : Activity() {
    private lateinit var settings: ConnectionSettings
    private lateinit var client: PhoneHistoryClient
    private lateinit var root: LinearLayout
    private lateinit var drawer: DrawerLayout
    private lateinit var navigation: LinearLayout
    private lateinit var title: TextView
    private lateinit var content: LinearLayout
    private lateinit var page: FrameLayout
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
    private var transcript: ListeningTranscriptView? = null
    private var selectedSession: String? = null
    private var selectedTitle = "Session"
    private var pending: Call? = null
    private var generation = 0
    private var nextBefore: String? = null
    private var sessions: JSONObject? = null
    private var exchanges = org.json.JSONArray()
    private var inputDecisions = org.json.JSONArray()
    private var inputBuffer = JSONObject()
    private var voiceActivity = org.json.JSONArray()
    private val turns = linkedMapOf<String, JSONObject>()
    private val expanded = mutableSetOf<String>()
    private val collapsed = mutableSetOf<String>()
    private var rendered = ""
    private val handler = Handler(Looper.getMainLooper())
    private val transcriptRefresh = object : Runnable {
        override fun run() {
            transcript?.refresh()
            if (tab == 0) status.text = settings.status.takeUnless { it.startsWith("Stopped") } ?: "Stopped"
            handler.postDelayed(this, 250)
        }
    }
    private val refresh = object : Runnable {
        override fun run() {
            val capture = runCatching { JSONObject(settings.captureStatus) }.getOrDefault(JSONObject())
            val fresh = runCatching { java.time.Instant.parse(capture.optString("updatedAt")).toEpochMilli() > System.currentTimeMillis() - 5000 }.getOrDefault(false)
            status.text = if (tab == 0) settings.status.takeUnless { it.startsWith("Stopped") } ?: "Stopped" else if (capture.optBoolean("recording") && fresh) settings.status else settings.listeningStopReason.takeIf { it.isNotBlank() } ?: capture.optString("error").takeUnless { it == "null" || it.isBlank() } ?: "Stopped"
            exchange?.text = settings.lastExchange.ifBlank { "Your conversation will appear here." }
            volume?.text = if (getSystemService(AudioManager::class.java).getStreamVolume(AudioManager.STREAM_MUSIC) == 0)
                "Media volume is muted" else "Speech follows your media volume"
            if (tab == 3) { renderLive(); loadLive() }
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
        val heading = LinearLayout(this).apply { gravity = Gravity.CENTER_VERTICAL }
        heading.addView(PhoneTheme.button(this, "☰") { drawer.openDrawer(GravityCompat.START) }.apply { contentDescription = "Open navigation" }, LinearLayout.LayoutParams(dp(48), dp(48)).apply { marginEnd = dp(12) })
        title = PhoneTheme.text(this, "Orchid", 24f, bold = true)
        heading.addView(title, LinearLayout.LayoutParams(0, -2, 1f)); header.addView(heading)
        status = PhoneTheme.text(this, settings.status, 13f, PhoneTheme.accent); header.addView(status)
        root.addView(header)
        scroll = ScrollView(this).apply { isFillViewport = true }
        content = PhoneTheme.column(this).apply { setPadding(dp(20), dp(8), dp(20), dp(20)) }
        scroll.addView(content)
        page = FrameLayout(this)
        root.addView(page, LinearLayout.LayoutParams(-1, 0, 1f))
        navigation = PhoneTheme.column(this).apply { setPadding(dp(16), dp(32), dp(16), dp(16)); setBackgroundColor(PhoneTheme.surface); fitsSystemWindows = true }
        listOf("Listen", "Sessions", "Settings", "Status").forEachIndexed { index, label ->
            navigation.addView(PhoneTheme.button(this, label) { switchTab(index); drawer.closeDrawer(GravityCompat.START) }, LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(8) })
        }
        drawer = DrawerLayout(this).apply {
            addView(root, DrawerLayout.LayoutParams(-1, -1))
            addView(navigation, DrawerLayout.LayoutParams(dp(280), -1, GravityCompat.START))
            setDrawerTitle(GravityCompat.START, "Orchid navigation")
        }
        setContentView(drawer); showTab()
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
        toggle.setTextColor(PhoneTheme.primary); toggle.gravity = GravityCompat.START or android.view.Gravity.CENTER_VERTICAL
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
        content.removeAllViews(); exchange = null; volume = null; composer = null; live = null; liveDetails = null; levelMeter = null; transcript = null; rendered = ""
        (content.parent as? android.view.ViewGroup)?.removeView(content)
        page.removeAllViews()
        if (tab == 0) page.addView(content, FrameLayout.LayoutParams(-1, -1))
        else {
            scroll.addView(content)
            page.addView(scroll, FrameLayout.LayoutParams(-1, -1))
            scroll.scrollTo(0, 0)
        }
        title.text = listOf("Orchid", "Sessions", "Settings", "Status")[tab]
        for (index in 0 until navigation.childCount) {
            val button = navigation.getChildAt(index) as Button
            button.background = PhoneTheme.background(this, if (index == tab) PhoneTheme.selected else PhoneTheme.surface)
            button.isSelected = index == tab
        }
        when (tab) { 0 -> listening(); 1 -> renderHistory(); 2 -> setup(); 3 -> conversation() }
    }
    private fun listening() {
        val actions = LinearLayout(this)
        actions.addView(PhoneTheme.button(this, "Start listening") { begin() }, LinearLayout.LayoutParams(0, -2, 1f).apply { marginEnd = dp(6) })
        actions.addView(PhoneTheme.button(this, "Stop listening") { startService(Intent(this, ConversationService::class.java).setAction("STOP")) }, LinearLayout.LayoutParams(0, -2, 1f).apply { marginStart = dp(6) })
        content.addView(actions)
        transcript = ListeningTranscriptView(this, settings).also { content.addView(it, LinearLayout.LayoutParams(-1, 0, 1f).apply { topMargin = dp(16) }) }
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
        diagnostics.addView(audioDeviceControls())
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
        section(content, "listening-timeout", "Listening timeout", true) { body ->
            addText(body, "Stop the microphone after this many seconds without input accepted as relevant by JEV. Replies remain available. Start listening or use your assistant shortcut to resume.", color = PhoneTheme.secondary)
            val timeout = EditText(this).apply { setSingleLine(); inputType = InputType.TYPE_CLASS_NUMBER; setText(settings.listeningTimeoutSeconds.toString()); hint = "Seconds"; contentDescription = "Listening timeout in seconds"; setTextColor(PhoneTheme.primary) }
            body.addView(timeout)
            action(body, "Save listening timeout") {
                val seconds = timeout.text.toString().trim().toIntOrNull()
                if (seconds == null || seconds <= 0) timeout.error = "Enter a positive number of seconds"
                else { settings.listeningTimeoutSeconds = seconds; Toast.makeText(this, "Listening timeout saved", Toast.LENGTH_SHORT).show() }
            }
        }
        section(content, "input-processing", "Input processing", true) { body ->
            fun number(label: String, value: Int): EditText {
                addText(body, label, 13f, PhoneTheme.secondary)
                return EditText(this).apply { setSingleLine(); inputType = InputType.TYPE_CLASS_NUMBER; setText(value.toString()); contentDescription = label; setTextColor(PhoneTheme.primary); body.addView(this) }
            }
            val sentences = number("Sentences per JEV query (1–20)", settings.sentencesPerQuery)
            val pause = number("Pause before JEV query · seconds (1–60)", settings.pauseSeconds)
            val context = number("Recent audio and text context · seconds (10–120)", settings.contextSeconds)
            val followup = number("Listening after delivery · seconds (1–600)", settings.followupSeconds)
            action(body, "Save input processing") {
                val fields = listOf(sentences to (1..20), pause to (1..60), context to (10..120), followup to (1..600))
                var valid = true
                fields.forEach { (field, range) -> if (field.text.toString().toIntOrNull() !in range) { field.error = "Enter a value in $range"; valid = false } }
                if (valid) {
                    settings.sentencesPerQuery = sentences.text.toString().toInt(); settings.pauseSeconds = pause.text.toString().toInt()
                    settings.contextSeconds = context.text.toString().toInt(); settings.followupSeconds = followup.text.toString().toInt()
                    startService(Intent(this, ConversationService::class.java).setAction("INPUT_SETTINGS"))
                    Toast.makeText(this, "Input processing saved", Toast.LENGTH_SHORT).show()
                }
            }
        }
        section(content, "assistant", "Assistant shortcuts", true) { body ->
            val role = getSystemService(android.app.role.RoleManager::class.java)
            addText(body, if (role.isRoleHeld(android.app.role.RoleManager.ROLE_ASSISTANT)) "Orchid is your phone assistant." else "Choose Orchid as your phone assistant to use the assistant gesture and connected device buttons.", color = PhoneTheme.secondary)
            action(body, "Choose phone assistant") {
                if (role.isRoleAvailable(android.app.role.RoleManager.ROLE_ASSISTANT) && !role.isRoleHeld(android.app.role.RoleManager.ROLE_ASSISTANT)) startActivityForResult(role.createRequestRoleIntent(android.app.role.RoleManager.ROLE_ASSISTANT), 3)
                else startActivity(Intent(android.provider.Settings.ACTION_MANAGE_DEFAULT_APPS_SETTINGS))
            }
            addText(body, "Garmin and headset buttons depend on the device's assistant support. Your microphone setting still applies. A custom wake phrase is not enabled.", color = PhoneTheme.secondary)
        }
        section(content, "google-handoff", "Hey Google handoff", true) { body ->
            body.addView(Switch(this).apply {
                text = "Automatic Google handoff"; setTextColor(PhoneTheme.primary); minHeight = dp(48)
                isChecked = settings.googleHandoff
                setOnCheckedChangeListener { _, checked -> settings.googleHandoff = checked }
            })
            addText(body, "Experimental: when Google's assistant overlay opens, dismiss it and start Orchid listening. Choose Google as the default assistant, enable Hey Google, and enable Orchid Google handoff in Accessibility.", color = PhoneTheme.secondary)
            action(body, "Enable handoff access") {
                startActivity(Intent(android.provider.Settings.ACTION_ACCESSIBILITY_SETTINGS))
            }
            addText(body, "Hey Google: Gemini → profile → Settings → Talk to Gemini hands-free → Hey Google and Voice Match. Phone and locked-screen handoff still need verification.", color = PhoneTheme.secondary)
        }
        section(content, "audio-devices", "Audio devices", true) { body ->
            body.addView(audioDeviceControls())
            addText(body, "Selections apply immediately. Phone and USB microphones preserve Bluetooth media playback quality. Status shows the actual routes.", color = PhoneTheme.secondary)
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
            voiceActivity = value.optJSONArray("activity") ?: org.json.JSONArray()
            if (selectedSession == null) sessions = value else {
                selectedTitle = value.optString("title", selectedTitle)
                exchanges = value.optJSONArray("exchanges") ?: org.json.JSONArray()
                inputDecisions = value.optJSONArray("inputDecisions") ?: org.json.JSONArray()
                inputBuffer = value.optJSONObject("inputBuffer") ?: JSONObject()
                if (before != null || turns.isEmpty()) nextBefore = value.optString("nextBefore").takeUnless { it == "null" || it.isBlank() }
                val entries = value.optJSONArray("turns")
                if (entries != null) for (index in 0 until entries.length()) { val turn = entries.getJSONObject(index); turns[turn.getString("id")] = turn }
            }
            val signature = if (selectedSession == null) sessions.toString() else turns.values.joinToString() + nextBefore + exchanges + inputDecisions + inputBuffer + voiceActivity
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
    private fun audioDeviceControls() = AudioDeviceControls(this, settings, { changeMicrophone(it) }, { value ->
        settings.audioOutput = value
        startService(Intent(this, ConversationService::class.java).setAction("AUDIO_OUTPUT"))
        renderLive()
    })
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
        if (recording && capture.has("idleRemainingSeconds")) lines += "Listening timeout in ${capture.optLong("idleRemainingSeconds")} seconds without relevant input"
        if (!recording && settings.listeningStopReason.isNotBlank()) lines += "${settings.listeningStopReason}. Resume here or through your assistant shortcut."
        if (recording && capture.optBoolean("microphoneUnavailable")) lines += "Selected microphone disconnected · using the phone microphone"
        val playback = runCatching { JSONObject(settings.playbackStatus) }.getOrDefault(JSONObject())
        lines += "Audio output: ${AudioDevices.label(settings.audioOutput)}"
        if (playback.optString("state") == "playing") lines += "Playing through: ${if (playback.optBoolean("routeActive")) playback.optString("routedOutput") else "Checking route"}"
        else if (!playback.isNull("routedDeviceKey")) lines += "Last playback through: ${playback.optString("routedOutput")}"
        if (settings.audioOutput != "automatic" && AudioDevices(this).output(settings.audioOutput) == null) lines += "Selected output disconnected · using Android media output"
        playback.optString("error").takeUnless { it == "null" || it.isBlank() }?.let { lines += "Playback error: $it" }
        capture.optString("error").takeUnless { it == "null" || it.isBlank() }?.let { lines += "Capture error: $it" }
        val level = if (recording) capture.optDouble("level", 0.0) else 0.0
        levelMeter?.progress = (kotlin.math.sqrt(level.coerceIn(0.0, 1.0)) * 100).toInt()
        lines += "Input signal: ${String.format(java.util.Locale.ROOT, "%.1f", level * 100)}% · ${if (recording && capture.optBoolean("speechDetected")) "speech detected" else "no speech detected"}"
        lines += "Audio packets: ${capture.optLong("segmentsCaptured")} recorded · ${capture.optLong("segmentsSent")} sent"
        if (liveError != null) lines += "Laptop status: $liveError"
        liveState?.let { state ->
            val audio = state.optJSONObject("audio") ?: JSONObject()
            lines += "Laptop: ${audio.optLong("received")} packets received · ${audio.optLong("transcribed")} context windows decoded · ${state.optInt("pendingAudio")} queued"
            if (audio.has("lastTranscript") && !audio.isNull("lastTranscript")) {
                val transcript = audio.optString("lastTranscript")
                if (transcript.isBlank()) lines += "Last audio: no words recognized · not sent to JEV"
                else details += "Last audio transcript: $transcript"
            }
            audio.optString("error").takeUnless { it == "null" || it.isBlank() }?.let { lines += "Transcription error: $it" }
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
            val tasks = sessions?.optJSONArray("tasks")
            val assignments = sessions?.optJSONArray("assignments")
            if (tasks != null && tasks.length() > 0) section(content, "voice-tasks", "Tasks", true) { body ->
                for (index in 0 until tasks.length()) {
                    val task = tasks.getJSONObject(index)
                    var latest: JSONObject? = null
                    if (assignments != null) for (i in 0 until assignments.length()) {
                        val assignment = assignments.getJSONObject(i)
                        if (assignment.optString("taskId") == task.optString("id")) latest = assignment
                    }
                    val current = latest
                    section(body, "task-${task.optString("id")}", "${task.optString("title")} · request ${task.optInt("revision")}") { details ->
                        val status = current?.optString("runtimeStatus")?.takeUnless { it == "null" || it.isBlank() } ?: "pending"
                        val delivery = current?.optJSONObject("delivery")
                        val amendment = delivery?.optString("inputId")?.takeUnless { it == "null" || it.isBlank() }
                        addText(details, status + if (amendment != null) " · amendment ${delivery.optString("state")}" else "", 13f, PhoneTheme.accent)
                        addText(details, task.optString("request"))
                        current?.optJSONObject("outcome")?.let { addText(details, "Reported outcome: ${it.optString("summary")}") }
                    }
                }
            }
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
                addText(body, if (inputBuffer.optBoolean("complete")) "Ready for reasoning" else "Waiting for the input to be complete", 13f, PhoneTheme.muted)
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
            for (turn in turns.values.sortedByDescending { it.optString("createdAt") }) {
                val id = turn.getString("id")
                val card = PhoneTheme.card(this)
                addText(card, "${turn.optString("status")} · ${turn.optString("createdAt").replace('T', ' ').take(19)}", 13f, PhoneTheme.accent)
                val userInput = turn.optString("provenance") == "user" || turn.optString("inputKind") == "speech"
                section(card, "$id-request", if (userInput) "Your input" else "Input / context", userInput) { addText(it, turn.optString("request")) }
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
        if (voiceActivity.length() > 0) section(content, "voice-activity", "Recent voice activity") { body ->
            for (index in 0 until voiceActivity.length()) {
                val entry = voiceActivity.getJSONObject(index)
                addText(body, "${entry.optString("createdAt").substringAfter('T').take(8)} · ${entry.optString("kind").replace('_', ' ')}", 12f, PhoneTheme.accent)
                entry.optString("text").takeIf { it.isNotBlank() }?.let { addText(body, it, 14f, PhoneTheme.secondary) }
            }
        }
        historyStatus = PhoneTheme.text(this, "Loading from laptop…", 13f, PhoneTheme.muted); content.addView(historyStatus)
        action(content, "Refresh") { loadHistory() }
    }
    private fun begin() {
        try { settings.validate() } catch (error: Exception) { Toast.makeText(this, error.message, Toast.LENGTH_LONG).show(); switchTab(2); return }
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) { requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 1); return }
        if (android.os.Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) { requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 2); return }
        switchTab(0); drawer.closeDrawer(GravityCompat.START)
        settings.status = "Connecting…"; status.text = settings.status
        startForegroundService(Intent(this, ConversationService::class.java).setAction("START"))
    }
    override fun onRequestPermissionsResult(code: Int, permissions: Array<out String>, results: IntArray) { super.onRequestPermissionsResult(code, permissions, results); if (results.isNotEmpty() && results[0] == PackageManager.PERMISSION_GRANTED) begin() }
    override fun onActivityResult(code: Int, result: Int, data: Intent?) { super.onActivityResult(code, result, data); if (code == 3 && tab == 2) showTab() }
    override fun onResume() { super.onResume(); handler.post(refresh); handler.post(transcriptRefresh) }
    override fun onPause() { handler.removeCallbacks(refresh); handler.removeCallbacks(transcriptRefresh); pending?.cancel(); pending = null; liveCall?.cancel(); liveCall = null; generation++; super.onPause() }
    override fun onDestroy() { client.close(); super.onDestroy() }
    override fun onSaveInstanceState(state: Bundle) { super.onSaveInstanceState(state); state.putInt("tab", tab); state.putString("session", selectedSession); state.putString("title", selectedTitle); state.putString("draft", composer?.text?.toString() ?: draft); state.putStringArrayList("expanded", ArrayList(expanded)); state.putStringArrayList("collapsed", ArrayList(collapsed)) }
    @Deprecated("Activity back navigation") override fun onBackPressed() { if (drawer.isDrawerOpen(GravityCompat.START)) drawer.closeDrawer(GravityCompat.START) else if (tab == 1 && selectedSession != null) { selectedSession = null; switchTab(1) } else if (tab != 0) switchTab(0) else super.onBackPressed() }
}
