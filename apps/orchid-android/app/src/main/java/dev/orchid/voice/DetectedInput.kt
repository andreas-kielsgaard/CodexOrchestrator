package dev.orchid.voice

import org.json.JSONArray
import org.json.JSONObject
import android.text.SpannableStringBuilder
import android.text.Spanned
import android.text.style.ForegroundColorSpan

/** Recent transcript blocks, before relevance filtering. */
class DetectedInput(private val settings: ConnectionSettings) {
    fun append(id: String, text: String) {
        val inputs = JSONArray(settings.detectedInput)
        val existing = (0 until inputs.length()).map { inputs.getJSONObject(it) }.firstOrNull { it.optString("id") == id }
        if (text.isNotBlank()) {
            if (existing != null) existing.put("text", text) else inputs.put(JSONObject().put("id", id).put("text", text))
        }
        save(inputs)
    }
    fun window(streamId: String, startMs: Long, blocks: JSONArray) {
        val inputs = JSONArray(settings.detectedInput)
        val incoming = (0 until blocks.length()).map { blocks.getJSONObject(it) }
        for (index in inputs.length() - 1 downTo 0) {
            val input = inputs.getJSONObject(index)
            if (input.optString("streamId") == streamId && input.optLong("endMs") >= startMs && incoming.none { it.getString("id") == input.getString("id") }) inputs.remove(index)
        }
        for (block in incoming) {
            val existing = (0 until inputs.length()).map { inputs.getJSONObject(it) }.firstOrNull { it.getString("id") == block.getString("id") }
            val target = existing ?: JSONObject().also { inputs.put(it) }
            target.put("id", block.getString("id")).put("streamId", streamId).put("text", block.getString("text"))
                .put("startMs", block.getLong("startMs")).put("endMs", block.getLong("endMs")).put("revision", block.getLong("revision"))
            if (!block.isNull("relevance")) target.put("relevance", sticky(target.optString("relevance"), block.getString("relevance")))
        }
        save(inputs)
    }
    private fun sticky(previous: String, next: String) = when {
        previous == "direct_request" || next == "direct_request" -> "direct_request"
        previous == "task_context" || next == "task_context" -> "task_context"
        else -> next
    }
    private fun save(inputs: JSONArray) {
        while (inputs.length() > 20) inputs.remove(0)
        settings.detectedInput = inputs.toString()
    }
    fun classify(id: String, relevance: String) {
        val inputs = JSONArray(settings.detectedInput)
        for (index in 0 until inputs.length()) {
            val input = inputs.getJSONObject(index)
            if (input.getString("id") == id) {
                input.put("relevance", sticky(input.optString("relevance"), relevance))
                settings.detectedInput = inputs.toString()
                return
            }
        }
    }
    fun display(): Spanned {
        val inputs = JSONArray(settings.detectedInput)
        val display = SpannableStringBuilder()
        for (index in 0 until inputs.length()) {
            if (index > 0) display.append("\n\n")
            val input = inputs.getJSONObject(index)
            val start = display.length
            display.append(input.getString("text"))
            val color = when (input.optString("relevance")) {
                "direct_request" -> PhoneTheme.accent
                "task_context" -> PhoneTheme.inputContext
                "unaddressed" -> PhoneTheme.muted
                else -> PhoneTheme.primary
            }
            display.setSpan(ForegroundColorSpan(color), start, display.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
        }
        return display
    }
}
