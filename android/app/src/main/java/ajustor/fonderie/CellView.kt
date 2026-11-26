package ajustor.fonderie

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.fonts.Font
import android.graphics.fonts.FontFamily
import android.os.Build
import android.view.SurfaceHolder
import android.view.SurfaceView

/**
 * Dessine la grille de cellules rendue par Rust : chaque cellule est un
 * rectangle de taille fixe avec son fond, son glyphe et sa couleur. Les
 * cellules gardent un rapport proche de 1:2, comme dans un terminal.
 */
class CellView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {
    val grid = CellGrid()

    var surfaceReady = false
        private set

    /** Taille d'une cellule en pixels et marge pour centrer la grille. */
    var cellW = 1f
        private set
    var cellH = 2f
        private set
    private var originX = 0f
    private var originY = 0f

    val regular: Typeface
    private val bold: Typeface
    private val text = Paint(Paint.ANTI_ALIAS_FLAG or Paint.SUBPIXEL_TEXT_FLAG or Paint.LINEAR_TEXT_FLAG)
    private val fill = Paint()
    private val boxes = BoxGlyphs()
    private val chars = CharArray(2)
    private val run = StringBuilder(64)
    private var baseline = 0f

    /** L'écran est gardé ici : seules les lignes modifiées sont redessinées. */
    private var bitmap: Bitmap? = null
    private var bitmapCanvas: Canvas? = null
    private var layoutKey = 0L

    init {
        holder.addCallback(this)
        regular = loadTypeface("fonts/JetBrainsMono-Regular.ttf")
        bold = loadTypeface("fonts/JetBrainsMono-Bold.ttf")
        text.typeface = regular
        text.textAlign = Paint.Align.LEFT
    }

    /** Police mono avec repli sur Noto Sans Symbols 2 (✦ ★…) puis Noto Sans Symbols (⚙). */
    private fun loadTypeface(path: String): Typeface {
        val assets = context.assets
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val main = FontFamily.Builder(Font.Builder(assets, path).build()).build()
            val symbols2 = FontFamily.Builder(
                Font.Builder(assets, "fonts/NotoSansSymbols2-Regular.ttf").build(),
            ).build()
            val symbols = FontFamily.Builder(
                Font.Builder(assets, "fonts/NotoSansSymbols-Regular.ttf").build(),
            ).build()
            return Typeface.CustomFallbackBuilder(main)
                .addCustomFallback(symbols2)
                .addCustomFallback(symbols)
                .setSystemFallback("monospace")
                .build()
        }
        return Typeface.createFromAsset(assets, path)
    }

    /** Nombre de lignes et colonnes pour `cols` colonnes sur la largeur de la vue. */
    fun layoutFor(cols: Int): Pair<Int, Int> {
        if (width == 0 || height == 0) return cols to 1
        cellW = width.toFloat() / cols
        cellH = cellW * 2f
        val rows = maxOf(1, (height / cellH).toInt())
        originX = (width - cols * cellW) / 2f
        originY = (height - rows * cellH) / 2f
        // Taille de police telle qu'un glyphe occupe la largeur d'une cellule
        // (mesurée à grande taille : à 1 px, l'arrondi fausse la mesure).
        text.typeface = regular
        text.textSize = MEASURE_SIZE
        val advance = text.measureText("M")
        text.textSize = MEASURE_SIZE * cellW / advance
        val fm = text.fontMetrics
        baseline = (cellH - (fm.descent + fm.ascent)) / 2f
        // Nouvelle géométrie : tout est à redessiner.
        val key = (cols.toLong() shl 40) or (rows.toLong() shl 20) or width.toLong() xor (height.toLong() shl 50)
        if (key != layoutKey) {
            layoutKey = key
            grid.markAll()
        }
        return cols to rows
    }

    /** Cellule sous un point de l'écran (bornée à la grille). */
    fun cellAt(px: Float, py: Float): Pair<Int, Int> {
        val col = ((px - originX) / cellW).toInt().coerceIn(0, maxOf(0, grid.cols - 1))
        val row = ((py - originY) / cellH).toInt().coerceIn(0, maxOf(0, grid.rows - 1))
        return col to row
    }

    fun render() {
        if (!surfaceReady || width == 0 || height == 0) return
        val off = ensureBitmap()
        val g = grid
        for (row in 0 until g.rows) {
            if (g.dirty[row]) {
                drawRow(off, row)
                g.dirty[row] = false
            }
        }
        val canvas = try {
            holder.lockHardwareCanvas()
        } catch (e: IllegalStateException) {
            null
        } ?: return
        try {
            canvas.drawBitmap(bitmap!!, 0f, 0f, null)
        } finally {
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun ensureBitmap(): Canvas {
        val b = bitmap
        if (b == null || b.width != width || b.height != height) {
            b?.recycle()
            val nb = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
            nb.eraseColor(Color.BLACK)
            bitmap = nb
            bitmapCanvas = Canvas(nb)
            grid.markAll()
        }
        return bitmapCanvas!!
    }

    /** Caractère de la police principale : se dessine en suite, d'un seul appel. */
    private fun isPlain(ch: Int) = ch in 0x21 until 0x2000

    private fun drawRow(canvas: Canvas, row: Int) {
        val g = grid
        val w = cellW
        val h = cellH
        val y = originY + row * h
        canvas.save()
        canvas.clipRect(0f, y, width.toFloat(), y + h)
        canvas.drawColor(Color.BLACK)
        // Fonds, regroupés par suites de même couleur.
        var col = 0
        while (col < g.cols) {
            val color = background(row * g.cols + col)
            var end = col + 1
            while (end < g.cols && background(row * g.cols + end) == color) end++
            if (color != 0) {
                fill.color = color
                canvas.drawRect(originX + col * w, y, originX + end * w, y + h, fill)
            }
            col = end
        }
        // Glyphes : suites de texte de même style, symboles un par un.
        col = 0
        while (col < g.cols) {
            val i = row * g.cols + col
            val ch = g.chars[i]
            if (ch == ' '.code || ch == 0) {
                col++
                continue
            }
            val m = g.mods[i].toInt()
            var color = foreground(i)
            if (m and CellGrid.DIM != 0) color = dim(color)
            val isBold = m and CellGrid.BOLD != 0
            val x = originX + col * w
            if (isPlain(ch)) {
                run.setLength(0)
                var end = col
                while (end < g.cols) {
                    val j = row * g.cols + end
                    val c = g.chars[j]
                    val mj = g.mods[j].toInt()
                    var cj = foreground(j)
                    if (mj and CellGrid.DIM != 0) cj = dim(cj)
                    val sameStyle = cj == color && (mj and CellGrid.BOLD != 0) == isBold
                    when {
                        c == ' '.code -> run.append(' ')
                        isPlain(c) && sameStyle -> run.appendCodePoint(c)
                        else -> break
                    }
                    end++
                }
                // Les espaces de fin ne dessinent rien.
                while (run.isNotEmpty() && run[run.length - 1] == ' ') {
                    run.setLength(run.length - 1)
                }
                text.color = color
                text.typeface = if (isBold) bold else regular
                canvas.drawText(run, 0, run.length, x, y + baseline, text)
                col = maxOf(end, col + 1)
                continue
            }
            text.color = color
            fill.color = color
            if (!boxes.draw(canvas, ch, x, y, w, h, fill)) {
                text.typeface = if (isBold) bold else regular
                val n = Character.toChars(ch, chars, 0)
                // Symbole d'une autre largeur : centré dans sa cellule.
                val adv = text.measureText(chars, 0, n)
                canvas.drawText(chars, 0, n, x + (w - adv) / 2f, y + baseline, text)
            }
            col++
        }
        canvas.restore()
    }

    private fun background(i: Int): Int {
        val reversed = grid.mods[i].toInt() and CellGrid.REVERSED != 0
        return if (reversed) orDefault(grid.fg[i], DEFAULT_FG) else grid.bg[i]
    }

    private fun foreground(i: Int): Int {
        val reversed = grid.mods[i].toInt() and CellGrid.REVERSED != 0
        return if (reversed) orDefault(grid.bg[i], Color.BLACK) else orDefault(grid.fg[i], DEFAULT_FG)
    }

    private fun orDefault(c: Int, default: Int) = if (c == 0) default else c

    private fun dim(c: Int): Int =
        Color.argb(Color.alpha(c), Color.red(c) * 6 / 10, Color.green(c) * 6 / 10, Color.blue(c) * 6 / 10)

    override fun surfaceCreated(holder: SurfaceHolder) {
        surfaceReady = true
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        surfaceReady = true
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        surfaceReady = false
    }

    companion object {
        private const val MEASURE_SIZE = 100f
        /** Couleur de texte par défaut (`theme::TEXT`). */
        val DEFAULT_FG: Int = Color.rgb(215, 215, 230)
    }
}
