package dev.orchid.voice

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.widget.*

/** Semantic colors shared with the desktop palette in src/theme/tokens.css. */
object PhoneTheme {
    val canvas = Color.parseColor("#141819")
    val surface = Color.parseColor("#1b2022")
    val raised = Color.parseColor("#22292c")
    val border = Color.parseColor("#39454a")
    val primary = Color.parseColor("#e5ebe8")
    val secondary = Color.parseColor("#bdc8c2")
    val muted = Color.parseColor("#a2b0a8")
    val accent = Color.parseColor("#8dd5b3")
    val inputContext = Color.parseColor("#8ab4f8")
    val selected = Color.parseColor("#233c33")
    val danger = Color.parseColor("#f0a5a0")
    fun dp(context: Context, value: Int) = (value * context.resources.displayMetrics.density).toInt()
    fun background(context: Context, color: Int = surface) = GradientDrawable().apply {
        setColor(color); cornerRadius = dp(context, 12).toFloat(); setStroke(dp(context, 1), border)
    }
    fun text(context: Context, value: String, size: Float = 15f, color: Int = primary, bold: Boolean = false) = TextView(context).apply {
        text = value; textSize = size; setTextColor(color); setLineSpacing(dp(context, 3).toFloat(), 1f)
        if (bold) setTypeface(typeface, Typeface.BOLD)
        setPadding(0, dp(context, 5), 0, dp(context, 5))
    }
    fun button(context: Context, label: String, action: () -> Unit) = Button(context).apply {
        text = label; isAllCaps = false; textSize = 14f; setTextColor(accent)
        minHeight = dp(context, 48); background = background(context, raised)
        setPadding(dp(context, 12), dp(context, 8), dp(context, 12), dp(context, 8))
        setOnClickListener { action() }
    }
    fun column(context: Context) = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL }
    fun card(context: Context) = column(context).apply {
        background = background(context); setPadding(dp(context, 16), dp(context, 12), dp(context, 16), dp(context, 12))
        layoutParams = LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(context, 12) }
    }
}
