package dev.orchid.voice

import android.content.Context
import android.graphics.*
import android.view.MotionEvent
import android.view.View
import org.json.JSONObject
import kotlin.math.*

/** Fits the saved design into one phone-sized map; selection exposes its immediate neighbors. */
class WorkflowMapView(context: Context, private val nodes: List<JSONObject>,
    private val edges: List<JSONObject>, private val selected: String?,
    private val select: (String) -> Unit) : View(context) {
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val boxes = linkedMapOf<String, RectF>()
    private val density = resources.displayMetrics.density
    init { contentDescription = "Workflow connection map. Select a component below for accessible navigation." }
    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        if (nodes.isEmpty()) return
        boxes.clear()
        val minX = nodes.minOf { it.optDouble("x", 0.0) }
        val minY = nodes.minOf { it.optDouble("y", 0.0) }
        val spanX = max(1.0, nodes.maxOf { it.optDouble("x", 0.0) } - minX)
        val spanY = max(1.0, nodes.maxOf { it.optDouble("y", 0.0) } - minY)
        val boxW = min(width / 3.25f, 110 * density)
        val boxH = 52 * density
        val pad = 5 * density
        for (node in nodes) {
            val x = pad + ((node.optDouble("x", 0.0) - minX) / spanX * (width - boxW - 2 * pad)).toFloat()
            val y = pad + ((node.optDouble("y", 0.0) - minY) / spanY * (height - boxH - 2 * pad)).toFloat()
            boxes[node.getString("id")] = RectF(x, y, x + boxW, y + boxH)
        }
        val sorted = edges.sortedBy { if (it.optString("source") == selected || it.optString("destination") == selected) 1 else 0 }
        for (edge in sorted) {
            val from = boxes[edge.optString("source")] ?: continue
            val to = boxes[edge.optString("destination")] ?: continue
            val active = edge.optString("source") == selected || edge.optString("destination") == selected
            paint.color = if (active) PhoneTheme.accent else PhoneTheme.muted
            paint.alpha = if (active || selected == null) 255 else 70
            paint.strokeWidth = (if (active) 2f else 1f) * density
            paint.style = Paint.Style.STROKE
            if (from == to) {
                val sx = from.left + from.width() * .3f
                val tx = from.left + from.width() * .7f
                val y = from.bottom
                canvas.drawPath(Path().apply { moveTo(sx, y); cubicTo(sx, y + 24 * density, tx, y + 24 * density, tx, y) }, paint)
                canvas.drawLine(tx - 4 * density, y + 4 * density, tx, y, paint)
                canvas.drawLine(tx + 4 * density, y + 4 * density, tx, y, paint)
                continue
            }
            if (abs(from.centerY() - to.centerY()) < density && from != to) {
                val direction = sign(to.centerX() - from.centerX())
                val sx = if (direction > 0) from.right else from.left
                val tx = if (direction > 0) to.left else to.right
                val y = from.centerY()
                canvas.drawLine(sx, y, tx, y, paint)
                canvas.drawLine(tx - direction * 4 * density, y - 4 * density, tx, y, paint)
                canvas.drawLine(tx - direction * 4 * density, y + 4 * density, tx, y, paint)
                continue
            }
            val downward = to.centerY() >= from.centerY()
            val sx = from.centerX(); val sy = if (downward) from.bottom else from.top
            val tx = to.centerX(); val ty = if (downward) to.top else to.bottom
            val bend = (if (downward) 20 else -20) * density
            val path = Path().apply { moveTo(sx, sy); cubicTo(sx, sy + bend, tx, ty - bend, tx, ty) }
            canvas.drawPath(path, paint)
            val arrow = 4 * density
            canvas.drawLine(tx - arrow, ty - sign(bend) * arrow, tx, ty, paint)
            canvas.drawLine(tx + arrow, ty - sign(bend) * arrow, tx, ty, paint)
        }
        for (node in nodes) {
            val box = boxes[node.getString("id")] ?: continue
            paint.style = Paint.Style.FILL; paint.alpha = 255
            paint.color = if (node.optString("id") == selected) PhoneTheme.selected else PhoneTheme.raised
            canvas.drawRoundRect(box, 8 * density, 8 * density, paint)
            paint.style = Paint.Style.STROKE
            paint.strokeWidth = (if (node.optString("id") == selected) 3f else 1.5f) * density
            paint.color = if (node.optString("kind") == "agent") PhoneTheme.inputContext else PhoneTheme.accent
            canvas.drawRoundRect(box, 8 * density, 8 * density, paint)
            paint.style = Paint.Style.FILL; paint.color = PhoneTheme.primary
            paint.textSize = 12 * resources.displayMetrics.scaledDensity
            paint.textAlign = Paint.Align.CENTER
            val lines = mutableListOf<String>(); var line = ""
            for (word in node.optString("name").split(' ')) {
                val candidate = if (line.isEmpty()) word else "$line $word"
                if (paint.measureText(candidate) > box.width() - 10 * density && line.isNotEmpty()) { lines.add(line); line = word } else line = candidate
            }
            if (line.isNotEmpty()) lines.add(line)
            val visible = lines.take(3)
            val lineHeight = 14 * density
            visible.forEachIndexed { i, label -> canvas.drawText(label, box.centerX(), box.centerY() + (i - (visible.size - 1) / 2f) * lineHeight + 4 * density, paint) }
        }
    }
    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.action == MotionEvent.ACTION_DOWN) return boxes.values.any { it.contains(event.x, event.y) }
        if (event.action == MotionEvent.ACTION_UP) {
            boxes.entries.firstOrNull { it.value.contains(event.x, event.y) }?.let { performClick(); select(it.key); return true }
        }
        return super.onTouchEvent(event)
    }
    override fun performClick(): Boolean { super.performClick(); return true }
}
