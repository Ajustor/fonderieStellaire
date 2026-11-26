package ajustor.fonderie

import java.io.File
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Aucun caractère affiché par le jeu ne doit tomber sur le carré de
 * remplacement : chacun est dessiné en primitives (blocs, cadres), ou présent
 * dans JetBrains Mono, ou dans Noto Sans Symbols 2 / Noto Sans Symbols (les
 * polices de repli, dans cet ordre).
 */
class GlyphCoverageTest {
    // java.awt.Font de la JVM de test (absent du classpath Android à la compilation).
    private val fontClass = Class.forName("java.awt.Font")
    private val createFont = fontClass.getMethod("createFont", Int::class.java, File::class.java)
    private val canDisplay = fontClass.getMethod("canDisplay", Int::class.java)
    private val trueType = fontClass.getField("TRUETYPE_FONT").getInt(null)

    private val fonts: List<Any> = listOf(
        "JetBrainsMono-Regular.ttf",
        "JetBrainsMono-Bold.ttf",
        "NotoSansSymbols2-Regular.ttf",
        "NotoSansSymbols-Regular.ttf",
    ).map { createFont.invoke(null, trueType, File("src/main/assets/fonts/$it")) }

    private fun covers(font: Any, cp: Int) = canDisplay.invoke(font, cp) as Boolean

    /** Caractères non ASCII des sources de la présentation (hors commentaires). */
    private fun usedGlyphs(): Map<Int, String> {
        val found = sortedMapOf<Int, String>()
        val sources = File("../../crates/presentation/src").walkTopDown().filter { it.extension == "rs" } +
            File("src/main/java").walkTopDown().filter { it.extension == "kt" }
        for (file in sources) {
            file.readLines().forEachIndexed { n, line ->
                val code = line.trim()
                if (code.startsWith("//") || code.startsWith("*") || code.startsWith("/*")) return@forEachIndexed
                var i = 0
                while (i < line.length) {
                    val cp = line.codePointAt(i)
                    // À partir de la ponctuation générale : les lettres accentuées
                    // du français sont couvertes par toute police latine.
                    if (cp >= 0x2000) found.putIfAbsent(cp, "${file.name}:${n + 1}")
                    i += Character.charCount(cp)
                }
            }
        }
        return found
    }

    @Test
    fun everyGlyphIsCovered() {
        val used = usedGlyphs()
        assertTrue("aucun glyphe trouvé : chemin des sources ?", used.size > 40)
        val missing = used.filterKeys { cp ->
            cp !in BoxGlyphs.PRIMITIVES && fonts.none { covers(it, cp) }
        }
        assertTrue(
            "glyphes sans police : " + missing.entries.joinToString { (cp, where) ->
                "${String(Character.toChars(cp))} (U+%04X, %s)".format(cp, where)
            },
            missing.isEmpty(),
        )
    }
}
