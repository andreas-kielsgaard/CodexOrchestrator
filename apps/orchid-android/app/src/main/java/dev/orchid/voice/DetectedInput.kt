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
        if ((0 until inputs.length()).any { inputs.getJSONObject(it).optString("id") == id }) return
        inputs.put(JSONObject().put("id", id).put("text", text))
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
