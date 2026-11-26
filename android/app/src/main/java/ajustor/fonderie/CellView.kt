package ajustor.fonderie

import android.content.Context
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
    private val text = Paint(Paint.ANTI_ALIAS_FLAG)
    private val fill = Paint()
    private val boxes = BoxGlyphs()
    private val chars = CharArray(2)
    private var baseline = 0f

    init {
        holder.addCallback(this)
        regular = loadTypeface("fonts/JetBrainsMono-Regular.ttf")
        bold = loadTypeface("fonts/JetBrainsMono-Bold.ttf")
        text.typeface = regular
        text.textAlign = Paint.Align.CENTER
    }

    /** Police mono avec repli sur Noto Sans Symbols 2 pour ✦ ★ ⚙… */
    private fun loadTypeface(path: String): Typeface {
        val assets = context.assets
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val main = FontFamily.Builder(Font.Builder(assets, path).build()).build()
            val symbols = FontFamily.Builder(
                Font.Builder(assets, "fonts/NotoSansSymbols2-Regular.ttf").build(),
            ).build()
            return Typeface.CustomFallbackBuilder(main)
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
        return cols to rows
    }

    /** Cellule sous un point de l'écran (bornée à la grille). */
    fun cellAt(px: Float, py: Float): Pair<Int, Int> {
        val col = ((px - originX) / cellW).toInt().coerceIn(0, maxOf(0, grid.cols - 1))
        val row = ((py - originY) / cellH).toInt().coerceIn(0, maxOf(0, grid.rows - 1))
        return col to row
    }

    fun render() {
        if (!surfaceReady) return
        val canvas = try {
            holder.lockHardwareCanvas()
        } catch (e: IllegalStateException) {
            null
        } ?: return
        try {
            drawGrid(canvas)
        } finally {
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun drawGrid(canvas: Canvas) {
        canvas.drawColor(Color.BLACK)
        val g = grid
        val w = cellW
        val h = cellH
        // Fonds, regroupés par suites de même couleur sur chaque ligne.
        for (row in 0 until g.rows) {
            val y = originY + row * h
            var col = 0
            while (col < g.cols) {
                val i = row * g.cols + col
                val color = background(i)
                var end = col + 1
                while (end < g.cols && background(row * g.cols + end) == color) end++
                if (color != 0) {
                    fill.color = color
                    canvas.drawRect(originX + col * w, y, originX + end * w, y + h + 0.5f, fill)
                }
                col = end
            }
        }
        // Glyphes, chacun centré dans sa cellule.
        for (row in 0 until g.rows) {
            val y = originY + row * h
            for (col in 0 until g.cols) {
                val i = row * g.cols + col
                val ch = g.chars[i]
                if (ch == ' '.code || ch == 0) continue
                val m = g.mods[i].toInt()
                var color = foreground(i)
                if (m and CellGrid.DIM != 0) color = dim(color)
                text.color = color
                fill.color = color
                val x = originX + col * w
                if (boxes.draw(canvas, ch, x, y, w, h, fill)) continue
                text.typeface = if (m and CellGrid.BOLD != 0) bold else regular
                val n = Character.toChars(ch, chars, 0)
                canvas.drawText(chars, 0, n, x + w / 2f, y + baseline, text)
            }
        }
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
