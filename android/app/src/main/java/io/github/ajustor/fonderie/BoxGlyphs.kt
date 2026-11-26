package io.github.ajustor.fonderie

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF

/**
 * Dessine les caractères de bloc et de cadre en primitives, pour qu'ils se
 * raccordent sans espace d'une cellule à l'autre (comme dans un bon émulateur
 * de terminal). Renvoie `false` si le caractère doit passer par la police.
 */
class BoxGlyphs {
    private val path = Path()
    private val rect = RectF()

    /** Bras d'un caractère de cadre : 0 absent, 1 fin, 2 épais/double. */
    private class Arms(val up: Int, val down: Int, val left: Int, val right: Int, val double: Boolean = false)

    private val lines: Map<Int, Arms> = mapOf(
        '─'.code to Arms(0, 0, 1, 1),
        '│'.code to Arms(1, 1, 0, 0),
        '┌'.code to Arms(0, 1, 0, 1),
        '┐'.code to Arms(0, 1, 1, 0),
        '└'.code to Arms(1, 0, 0, 1),
        '┘'.code to Arms(1, 0, 1, 0),
        '├'.code to Arms(1, 1, 0, 1),
        '┤'.code to Arms(1, 1, 1, 0),
        '┬'.code to Arms(0, 1, 1, 1),
        '┴'.code to Arms(1, 0, 1, 1),
        '┼'.code to Arms(1, 1, 1, 1),
        '━'.code to Arms(0, 0, 2, 2),
        '┃'.code to Arms(2, 2, 0, 0),
        '═'.code to Arms(0, 0, 2, 2, true),
        '║'.code to Arms(2, 2, 0, 0, true),
        '╔'.code to Arms(0, 2, 0, 2, true),
        '╗'.code to Arms(0, 2, 2, 0, true),
        '╚'.code to Arms(2, 0, 0, 2, true),
        '╝'.code to Arms(2, 0, 2, 0, true),
    )

    fun draw(canvas: Canvas, ch: Int, x: Float, y: Float, w: Float, h: Float, paint: Paint): Boolean {
        when (ch) {
            '█'.code -> fill(canvas, x, y, x + w, y + h, paint)
            '▀'.code -> fill(canvas, x, y, x + w, y + h / 2f, paint)
            '▌'.code -> fill(canvas, x, y, x + w / 2f, y + h, paint)
            '▐'.code -> fill(canvas, x + w / 2f, y, x + w, y + h, paint)
            in 0x2581..0x2587 -> {
                // ▁ à ▇ : huitièmes depuis le bas.
                val eighths = ch - 0x2580
                fill(canvas, x, y + h * (8 - eighths) / 8f, x + w, y + h, paint)
            }
            '░'.code, '▒'.code, '▓'.code -> {
                val alpha = when (ch) {
                    '░'.code -> 0.25f
                    '▒'.code -> 0.5f
                    else -> 0.75f
                }
                val old = paint.alpha
                paint.alpha = (old * alpha).toInt()
                fill(canvas, x, y, x + w, y + h, paint)
                paint.alpha = old
            }
            '▚'.code -> {
                fill(canvas, x, y, x + w / 2f, y + h / 2f, paint)
                fill(canvas, x + w / 2f, y + h / 2f, x + w, y + h, paint)
            }
            '▞'.code -> {
                fill(canvas, x + w / 2f, y, x + w, y + h / 2f, paint)
                fill(canvas, x, y + h / 2f, x + w / 2f, y + h, paint)
            }
            '╭'.code, '╮'.code, '╰'.code, '╯'.code -> rounded(canvas, ch, x, y, w, h, paint)
            else -> {
                val arms = lines[ch] ?: return false
                drawArms(canvas, arms, x, y, w, h, paint)
            }
        }
        return true
    }

    private fun fill(canvas: Canvas, l: Float, t: Float, r: Float, b: Float, paint: Paint) {
        paint.style = Paint.Style.FILL
        canvas.drawRect(l, t, r, b, paint)
    }

    private fun stroke(w: Float, weight: Int): Float = maxOf(1f, w / 8f) * (if (weight == 2) 2f else 1f)

    private fun drawArms(canvas: Canvas, a: Arms, x: Float, y: Float, w: Float, h: Float, paint: Paint) {
        val cx = x + w / 2f
        val cy = y + h / 2f
        if (a.double) {
            // Deux traits fins parallèles.
            val t = stroke(w, 1)
            val gap = w / 5f
            for (o in floatArrayOf(-gap, gap)) {
                if (a.left > 0 || a.right > 0) {
                    val l = if (a.left > 0) x else cx - gap
                    val r = if (a.right > 0) x + w else cx + gap
                    fill(canvas, l, cy + o - t / 2f, r + t / 2f, cy + o + t / 2f, paint)
                }
                if (a.up > 0 || a.down > 0) {
                    val top = if (a.up > 0) y else cy - gap
                    val bottom = if (a.down > 0) y + h else cy + gap
                    fill(canvas, cx + o - t / 2f, top, cx + o + t / 2f, bottom + t / 2f, paint)
                }
            }
            return
        }
        if (a.left > 0) {
            val t = stroke(w, a.left)
            fill(canvas, x, cy - t / 2f, cx + t / 2f, cy + t / 2f, paint)
        }
        if (a.right > 0) {
            val t = stroke(w, a.right)
            fill(canvas, cx - t / 2f, cy - t / 2f, x + w, cy + t / 2f, paint)
        }
        if (a.up > 0) {
            val t = stroke(w, a.up)
            fill(canvas, cx - t / 2f, y, cx + t / 2f, cy + t / 2f, paint)
        }
        if (a.down > 0) {
            val t = stroke(w, a.down)
            fill(canvas, cx - t / 2f, cy - t / 2f, cx + t / 2f, y + h, paint)
        }
    }

    private fun rounded(canvas: Canvas, ch: Int, x: Float, y: Float, w: Float, h: Float, paint: Paint) {
        val cx = x + w / 2f
        val cy = y + h / 2f
        val r = w / 2f
        path.reset()
        when (ch) {
            '╭'.code -> {
                path.moveTo(x + w, cy)
                rect.set(cx, cy, cx + 2 * r, cy + 2 * r)
                path.arcTo(rect, 270f, -90f)
                path.lineTo(cx, y + h)
            }
            '╮'.code -> {
                path.moveTo(x, cy)
                rect.set(cx - 2 * r, cy, cx, cy + 2 * r)
                path.arcTo(rect, 270f, 90f)
                path.lineTo(cx, y + h)
            }
            '╰'.code -> {
                path.moveTo(cx, y)
                path.lineTo(cx, cy - r)
                rect.set(cx, cy - 2 * r, cx + 2 * r, cy)
                path.arcTo(rect, 180f, -90f)
                path.lineTo(x + w, cy)
            }
            else -> {
                path.moveTo(cx, y)
                path.lineTo(cx, cy - r)
                rect.set(cx - 2 * r, cy - 2 * r, cx, cy)
                path.arcTo(rect, 0f, 90f)
                path.lineTo(x, cy)
            }
        }
        paint.style = Paint.Style.STROKE
        paint.strokeWidth = stroke(w, 1)
        canvas.drawPath(path, paint)
        paint.style = Paint.Style.FILL
    }
}
