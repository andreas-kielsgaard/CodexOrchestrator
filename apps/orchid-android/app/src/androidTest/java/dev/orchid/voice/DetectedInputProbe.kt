package dev.orchid.voice

import android.content.Context
import android.os.Bundle
import android.text.style.ForegroundColorSpan
import org.json.JSONArray
import org.json.JSONObject

object DetectedInputProbe {
    fun run(context: Context, result: Bundle) {
        val settings = ConnectionSettings(context)
        val original = settings.detectedInput
        try {
            settings.detectedInput = "[]"
            val inputs = DetectedInput(settings)
            fun sentence(id: String, text: String, revision: Int, at: Long) = JSONObject()
                .put("id", id).put("text", text).put("revision", revision).put("startMs", at).put("endMs", at + 1000)
            inputs.window("stream", 0, JSONArray().put(sentence("one", "Orchid", 1, 0)))
            inputs.classify("one", "direct_request")
            inputs.window("stream", 0, JSONArray().put(sentence("one", "Orchid, check this please.", 2, 0)).put(sentence("two", "More detail.", 1, 1000)))
            check(JSONArray(settings.detectedInput).length() == 2)
            inputs.classify("one", "unaddressed"); inputs.classify("two", "task_context")
            val display = inputs.display()
            check(display.toString() == "Orchid, check this please.\n\nMore detail.")
            val colors = display.getSpans(0, display.length, ForegroundColorSpan::class.java).map { it.foregroundColor }
            check(colors == listOf(PhoneTheme.accent, PhoneTheme.inputContext))
            inputs.window("stream", 0, JSONArray().put(sentence("one", "Revised again.", 3, 0)))
            check(JSONArray(settings.detectedInput).length() == 1) { "Removed sentence remained visible" }
            inputs.window("stream", 0, JSONArray())
            check(JSONArray(settings.detectedInput).length() == 0)
            for (index in 0 until 25) inputs.append("bounded-$index", "Sentence $index.")
            check(JSONArray(settings.detectedInput).length() == 20)
            result.putString("transcripts", "Sentence revisions, removals, sticky classification colors and rolling limit passed")
        } finally {
            context.getSharedPreferences("connection", Context.MODE_PRIVATE).edit().putString("detected-input", original).commit()
        }
    }
}
