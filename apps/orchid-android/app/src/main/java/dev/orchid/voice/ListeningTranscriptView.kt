package dev.orchid.voice

import android.content.Context
import android.view.View
import android.view.ViewTreeObserver
import android.widget.ScrollView

class ListeningTranscriptView(context: Context, private val settings: ConnectionSettings) : ScrollView(context) {
    private val text = PhoneTheme.text(context, "", 16f).apply {
        setTextIsSelectable(true)
        contentDescription = "Latest detected input before filtering"
        val padding = PhoneTheme.dp(context, 12)
        setPadding(padding, padding, padding, padding)
    }
    private var rendered: String? = null
    init {
        isFillViewport = true
        background = PhoneTheme.background(context, PhoneTheme.surface)
        addView(text)
        refresh()
    }
    fun refresh() {
        val input = settings.detectedInput
        if (rendered == input) return
        val following = !canScrollVertically(1)
        text.text = DetectedInput(settings).display()
        rendered = input
        if (following) viewTreeObserver.addOnGlobalLayoutListener(object : ViewTreeObserver.OnGlobalLayoutListener {
            override fun onGlobalLayout() {
                viewTreeObserver.removeOnGlobalLayoutListener(this)
                fullScroll(View.FOCUS_DOWN)
            }
        })
    }
}
