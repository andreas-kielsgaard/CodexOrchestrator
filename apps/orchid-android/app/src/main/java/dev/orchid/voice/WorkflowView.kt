package dev.orchid.voice

import android.content.Context
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import okhttp3.Call
import org.json.JSONArray
import org.json.JSONObject

/** A small read-only projection of the active instance, not a second workflow editor. */
class WorkflowView(context: Context, private val client: PhoneHistoryClient, private val scroll: ScrollView,
    private val openSession: (String, String) -> Unit) : LinearLayout(context) {
    private var call: Call? = null
    private var epoch = 0
    private var data: JSONObject? = null
    private var signature = ""
    private var routes = false
    private var selectedNode: String? = null
    private val expanded = mutableSetOf<String>()
    private val status = PhoneTheme.text(context, "Loading active workflow…", 13f, PhoneTheme.muted)
    private val body = PhoneTheme.column(context)
    init {
        orientation = VERTICAL
        addView(status)
        val tabs = LinearLayout(context)
        tabs.addView(PhoneTheme.button(context, "Components") { routes = false; selectedNode = null; scroll.scrollTo(0, 0); render() }, LayoutParams(0, -2, 1f))
        tabs.addView(PhoneTheme.button(context, "Activity") { routes = true; selectedNode = null; scroll.scrollTo(0, 0); render() }, LayoutParams(0, -2, 1f))
        addView(tabs); addView(body); refresh()
    }
    fun cancel() { epoch++; call?.cancel(); call = null }
    fun refresh() {
        if (call != null) return
        val expected = epoch
        call = client.get("/workflow") { value, error ->
            if (epoch != expected) return@get
            call = null
            if (error != null || value == null) {
                status.text = "${error ?: "Workflow unavailable"}${if (data != null) "\nShowing last received state." else ""}"
                status.setTextColor(PhoneTheme.danger); return@get
            }
            data = value
            status.text = "Live · ${value.optString("updatedAt").substringAfter('T').take(8)} UTC · read only"
            status.setTextColor(PhoneTheme.muted)
            val next = value.optJSONObject("instance").toString() + value.optJSONArray("nodes") + value.optJSONArray("connections") + value.optJSONArray("attempts") + value.optJSONObject("diagnostics")?.optJSONObject("input")
            if (next != signature) { signature = next; render() }
        }
    }
    private fun items(array: JSONArray?): List<JSONObject> = if (array == null) emptyList() else (0 until array.length()).mapNotNull { array.optJSONObject(it) }
    private fun text(parent: LinearLayout, value: String, color: Int = PhoneTheme.primary, size: Float = 14f) {
        parent.addView(PhoneTheme.text(context, value, size, color).apply { setTextIsSelectable(true) })
    }
    private fun expandable(key: String, title: String, detail: String, build: (LinearLayout) -> Unit) {
        val card = PhoneTheme.card(context)
        val contents = PhoneTheme.column(context)
        val toggle = PhoneTheme.button(context, "${if (key in expanded) "▾" else "▸"} $title") {}
        toggle.contentDescription = title
        toggle.setOnClickListener {
            if (!expanded.add(key)) expanded.remove(key)
            contents.visibility = if (key in expanded) View.VISIBLE else View.GONE
            toggle.text = "${if (key in expanded) "▾" else "▸"} $title"
        }
        card.addView(toggle); text(card, detail, PhoneTheme.secondary, 13f)
        build(contents); contents.visibility = if (key in expanded) View.VISIBLE else View.GONE
        card.addView(contents); body.addView(card)
    }
    private fun session(parent: LinearLayout, id: String, title: String) {
        parent.addView(PhoneTheme.button(context, "Open $title") { openSession(id, title) })
    }
    private fun meaning(trigger: String): String = when (trigger) {
        "transcript.window" -> "Speech ready to check"
        "evaluate.evaluated" -> "Speech classified"
        "accept_evaluation.work_ready" -> "Accepted work request"
        "accept_evaluation.reaction_ready" -> "Direct conversational reply"
        "acknowledge.requested" -> "Immediate acknowledgment"
        "delegate_work.requested" -> "Work assigned"
        "report_outcome.outcome" -> "Work result returned"
        "report_progress.progress" -> "Progress update"
        "request_output.requested" -> "Useful update ready to say"
        "publish.requested" -> "Reply ready to speak"
        "route_input.rerouted" -> "Needs work or deeper handling"
        "on_invocation_completed.completed" -> "Agent turn finished"
        "speak.published" -> "Reply published"
        "observe.observed" -> "Watched session changed"
        else -> trigger.replace('_', ' ')
    }
    private fun render() {
        val y = scroll.scrollY
        body.removeAllViews()
        val value = data ?: return
        val instance = value.optJSONObject("instance")
        if (instance == null) { text(body, "No Voice workflow instance selected. Select one in Orchid on the host."); return }
        text(body, instance.optString("name"), size = 20f)
        text(body, "${instance.optString("design")} · revision ${instance.optInt("revision")}", PhoneTheme.secondary)
        val nodes = items(value.optJSONArray("nodes"))
        val names = nodes.associate { it.optString("id") to it.optString("name") }
        val edges = items(value.optJSONArray("connections"))
        val attempts = items(value.optJSONArray("attempts"))
        val diagnostics = value.optJSONObject("diagnostics")
        if (routes) diagnostics?.optJSONObject("input")?.let { text(body, "${it.optString("title")}\n${it.optString("detail")}", PhoneTheme.accent) }
        val errors = listOf("error", "filterError").mapNotNull { key -> diagnostics?.takeUnless { it.isNull(key) }?.optString(key)?.takeIf { it.isNotBlank() } }
        if (routes) errors.forEach { text(body, it, PhoneTheme.danger) }
        else if (errors.isNotEmpty()) body.addView(PhoneTheme.button(context, "Needs attention · open Activity") { routes = true; render() })
        if (!routes) {
            val selected = nodes.find { it.optString("id") == selectedNode }
            text(body, selected?.optString("name") ?: "The whole workflow", size = 18f)
            text(body, selected?.optString("role")?.substringBefore(". ") ?: "Blue: agents · Green: services. Tap to trace connections.", PhoneTheme.secondary, 13f)
            fun select(id: String) { selectedNode = id; expanded.add("node:$id"); render() }
            body.addView(WorkflowMapView(context, nodes, edges, selectedNode, ::select), LayoutParams(-1, PhoneTheme.dp(context, 360)))
            val picker = android.widget.Spinner(context)
            val labels = listOf("Choose a component") + nodes.map { it.optString("name") }
            picker.adapter = android.widget.ArrayAdapter(context, android.R.layout.simple_spinner_dropdown_item, labels)
            picker.setSelection(nodes.indexOfFirst { it.optString("id") == selectedNode } + 1)
            picker.onItemSelectedListener = object : android.widget.AdapterView.OnItemSelectedListener {
                override fun onNothingSelected(parent: android.widget.AdapterView<*>?) {}
                override fun onItemSelected(parent: android.widget.AdapterView<*>?, view: View?, position: Int, id: Long) {
                    if (position > 0 && nodes[position - 1].optString("id") != selectedNode) select(nodes[position - 1].getString("id"))
                }
            }
            body.addView(picker)
            for (node in nodes.filter { it.optString("id") == selectedNode }) {
                val id = node.getString("id")
                val sessions = items(node.optJSONArray("sessions"))
                val operations = attempts.filter { it.optString("destinationNodeId") == id }
                val state = if (node.optString("kind") == "agent") {
                    if (sessions.isEmpty()) "No session yet" else sessions.groupingBy { it.optString("status").takeUnless { s -> s == "null" } ?: "idle" }.eachCount().entries.joinToString(" · ") { "${it.value} ${it.key}" }
                } else operations.firstOrNull()?.let { if (!it.isNull("error")) "Failed" else if (it.isNull("completedAt")) "Processing" else "Last operation completed" } ?: "Waiting for an event"
                val outgoing = edges.filter { it.optString("source") == id }
                val destinations = outgoing.mapNotNull { names[it.optString("destination")] }.distinct()
                val role = node.optString("role").takeIf { it.isNotBlank() } ?: "Role supplied by the workflow design."
                val shortRole = role.substringBefore(". ").let { if (it.endsWith('.')) it else "$it." }
                expandable("node:$id", node.optString("name"), "$shortRole\n\n${if (destinations.isEmpty()) "End of this path" else "→ " + destinations.joinToString(" · ")}\n$state") { detail ->
                    if (shortRole != role) text(detail, role, PhoneTheme.secondary)
                    if (!node.isNull("model")) text(detail, "${node.optString("model")} · ${node.optString("reasoning")} reasoning", PhoneTheme.muted, 12f)
                    fun link(edge: JSONObject, incoming: Boolean) {
                        val target = edge.optString(if (incoming) "source" else "destination")
                        val label = "${if (incoming) "←" else "→"} ${names[target]} · ${meaning(edge.optString("trigger"))}"
                        detail.addView(PhoneTheme.button(context, label) { selectedNode = target; expanded.add("node:$target"); scroll.scrollTo(0, 0); render() })
                    }
                    text(detail, "Receives", PhoneTheme.accent)
                    val incoming = edges.filter { it.optString("destination") == id }
                    if (incoming.isEmpty()) text(detail, "Starts from an external event", PhoneTheme.secondary)
                    incoming.forEach { link(it, true) }
                    text(detail, "Sends", PhoneTheme.accent)
                    outgoing.forEach { link(it, false) }
                    sessions.forEach { entry ->
                        text(detail, "${entry.optString("title")} · ${entry.optString("status")}")
                        if (!entry.isNull("error")) text(detail, entry.optString("error"), PhoneTheme.danger)
                        session(detail, entry.getString("id"), entry.optString("title"))
                    }
                    detail.addView(PhoneTheme.button(context, "Activity for this component") { routes = true; selectedNode = id; scroll.scrollTo(0, 0); render() })
                }
            }
        } else {
            text(body, "${attempts.size} recent operations · ${value.optInt("totalAttempts")} recorded. Delivery acceptance is separate from an agent finishing.", PhoneTheme.secondary, 13f)
            for (edge in edges.filter { selectedNode == null || it.optString("source") == selectedNode || it.optString("destination") == selectedNode }) {
                val id = edge.getString("id")
                val matching = attempts.filter { it.optString("connectionId") == id }
                val title = "${names[edge.optString("source")]} → ${names[edge.optString("destination")]}"
                expandable("edge:$id", title, "${meaning(edge.optString("trigger"))}\n${matching.size} operations in the recent window") { detail ->
                    if (matching.isEmpty()) text(detail, "No recorded delivery in the recent window.", PhoneTheme.muted)
                    for (attempt in matching) {
                        val state = if (!attempt.isNull("error")) "Failed" else if (attempt.isNull("completedAt")) "Processing" else "Operation completed"
                        text(detail, "${attempt.optString("createdAt")} · $state", PhoneTheme.accent)
                        if (!attempt.isNull("error")) text(detail, attempt.optString("error"), PhoneTheme.danger)
                        for (delivery in items(attempt.optJSONArray("deliveries"))) {
                            val outcome = delivery.optJSONObject("outcome") ?: JSONObject()
                            val receipt = if (outcome.optString("kind") == "failed") outcome.optString("message") else outcome.optString("state").takeUnless { it == "null" || it.isBlank() } ?: "dispatched"
                            text(detail, "Delivery: $receipt", if (outcome.optString("kind") == "failed") PhoneTheme.danger else PhoneTheme.primary)
                            session(detail, delivery.getString("sessionId"), "recipient conversation")
                        }
                        text(detail, "Input\n${attempt.optString("input")}\nOutputs\n${attempt.optString("outputs")}", PhoneTheme.secondary, 12f)
                    }
                }
            }
        }
        scroll.post { scroll.scrollTo(0, y) }
    }
}
