package io.github.ajustor.fonderie

import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Copie locale de l'écran rendu par Rust : une cellule = un caractère, une
 * couleur de texte, une couleur de fond (ARGB, 0 = couleur par défaut) et des
 * modificateurs. `apply` reçoit seulement les lignes modifiées (format décrit
 * dans `presentation::cells`).
 */
class CellGrid {
    var cols = 0
        private set
    var rows = 0
        private set
    var chars = IntArray(0)
        private set
    var fg = IntArray(0)
        private set
    var bg = IntArray(0)
        private set
    var mods = ByteArray(0)
        private set

    fun apply(bytes: ByteArray) {
        val buf = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
        val c = buf.short.toInt() and 0xFFFF
        val r = buf.short.toInt() and 0xFFFF
        val sent = buf.short.toInt() and 0xFFFF
        if (c != cols || r != rows) resize(c, r)
        repeat(sent) {
            val row = buf.short.toInt() and 0xFFFF
            var i = row * cols
            repeat(cols) {
                chars[i] = buf.int
                fg[i] = buf.int
                bg[i] = buf.int
                mods[i] = buf.get()
                i++
            }
        }
    }

    private fun resize(c: Int, r: Int) {
        cols = c
        rows = r
        val n = c * r
        chars = IntArray(n) { ' '.code }
        fg = IntArray(n)
        bg = IntArray(n)
        mods = ByteArray(n)
    }

    companion object {
        const val BOLD: Int = 1
        const val DIM: Int = 2
        const val REVERSED: Int = 16
    }
}
