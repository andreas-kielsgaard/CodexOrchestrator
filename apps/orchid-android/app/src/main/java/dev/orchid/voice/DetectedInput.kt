package dev.orchid.voice

import org.json.JSONArray
import org.json.JSONObject
import android.text.SpannableStringBuilder
import android.text.Spanned
import android.text.style.ForegroundColorSpan

/** Recent transcript blocks, before relevance filtering. */
class DetectedInput(private val settings: ConnectionSettings) {
    fun append(id: String, text: String) {
        update(id, text, true)
    }
    fun preview(id: String, text: String) {
        if (text.isNotBlank()) update(id, text, false)
    }
    private fun update(id: String, text: String, finalized: Boolean) {
        val inputs = JSONArray(settings.detectedInput)
        val existing = (0 until inputs.length()).map { inputs.getJSONObject(it) }.firstOrNull { it.optString("id") == id }
        if (existing != null) {
            if (!finalized && existing.optBoolean("finalized", true)) return
            existing.put("text", text).put("finalized", finalized)
        } else inputs.put(JSONObject().put("id", id).put("text", text).put("finalized", finalized))
        while (inputs.length() > 20) inputs.remove(0)
        settings.detectedInput = inputs.toString()
    }
    fun classify(id: String, relevance: String) {
        val inputs = JSONArray(settings.detectedInput)
        for (index in 0 until inputs.length()) {
            val input = inputs.getJSONObject(index)
            if (input.getString("id") == id) {
                input.put("relevance", relevance)
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
