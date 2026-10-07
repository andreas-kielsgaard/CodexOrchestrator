package dev.orchid.voice

import android.content.Context
import android.os.Bundle
import android.text.style.ForegroundColorSpan
import org.json.JSONArray

object DetectedInputProbe {
    fun run(context: Context, result: Bundle) {
        val settings = ConnectionSettings(context)
        val original = settings.detectedInput
        try {
            settings.detectedInput = "[]"
            val inputs = DetectedInput(settings)
            inputs.preview("live", "Orchid")
            inputs.preview("live", "Orchid, check this")
            check(JSONArray(settings.detectedInput).length() == 1)
            check(inputs.display().toString() == "Orchid, check this")
            inputs.append("live", "Orchid, check this please.")
            inputs.classify("live", "direct_request")
            inputs.preview("live", "stale draft")
            val displayed = inputs.display()
            check(displayed.toString() == "Orchid, check this please.")
            check(displayed.getSpans(0, displayed.length, ForegroundColorSpan::class.java).single().foregroundColor == PhoneTheme.accent)
            inputs.append("context", "More detail")
            inputs.classify("context", "task_context")
            inputs.append("ambient", "Other conversation")
            inputs.classify("ambient", "unaddressed")
            val colors = inputs.display().let { it.getSpans(0, it.length, ForegroundColorSpan::class.java).map { span -> span.foregroundColor } }
            check(colors == listOf(PhoneTheme.accent, PhoneTheme.inputContext, PhoneTheme.muted))
            for (index in 0 until 25) inputs.preview("bounded-$index", "draft $index")
            check(JSONArray(settings.detectedInput).length() == 20)
            result.putString("transcripts", "Draft replacement, finalization, late-draft rejection, classification colors, and rolling limit passed")
        } finally {
            settings.detectedInput = original
            context.getSharedPreferences("connection", Context.MODE_PRIVATE).edit().putString("detected-input", original).commit()
        }
    }
}
