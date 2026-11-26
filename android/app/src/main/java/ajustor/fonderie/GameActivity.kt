package ajustor.fonderie

import android.os.Bundle
import android.util.Log
import android.view.Choreographer
import android.view.MotionEvent
import android.view.VelocityTracker
import android.view.ViewConfiguration
import androidx.activity.ComponentActivity
import androidx.activity.OnBackPressedCallback
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.viewinterop.AndroidView
import uniffi.fonderie.Game
import uniffi.fonderie.Request
import uniffi.fonderie.TabInfo
import kotlin.math.abs
import kotlin.math.exp

/**
 * Cycle de vie, boucle d'images et gestes. À chaque image, Rust avance le jeu
 * et renvoie les cellules modifiées, que `CellView` dessine ; la barre
 * d'onglets native est au-dessous.
 */
class GameActivity : ComponentActivity(), Choreographer.FrameCallback {
    private lateinit var game: Game
    private lateinit var view: CellView

    private var running = false
    private var lastFrameNanos = 0L
    private var firstResume = true

    private val tabs = mutableStateOf<List<TabInfo>>(emptyList())
    private val barVisible = mutableStateOf(false)

    // Geste en cours.
    /** CAPTURED : la zone touchée suit le doigt elle-même (curseur, carte). */
    private enum class Gesture { NONE, PRESS, CAPTURED, SCROLL, SWIPE }
    private var gesture = Gesture.NONE
    private var downX = 0f
    private var downY = 0f
    private var lastY = 0f
    private var scrollAcc = 0f
    private var velocity: VelocityTracker? = null
    private var touchSlop = 0

    /** Inertie du défilement, en lignes par seconde. */
    private var flingRows = 0f
    private var flingAcc = 0f

    private val exportLauncher =
        registerForActivityResult(ActivityResultContracts.CreateDocument("application/json")) { uri ->
            if (uri == null) return@registerForActivityResult
            runCatching {
                val bytes = game.exportSave()
                contentResolver.openOutputStream(uri)?.use { it.write(bytes) }
            }.onSuccess {
                game.notify("✦ Sauvegarde exportée", "À reprendre sur ordinateur avec --import")
            }.onFailure {
                game.notify("Export impossible", it.message ?: "erreur inconnue")
            }
        }

    private val importLauncher =
        registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
            if (uri == null) return@registerForActivityResult
            runCatching {
                val bytes = contentResolver.openInputStream(uri)?.use { it.readBytes() }
                    ?: error("fichier illisible")
                game.importSave(bytes)
            }.onSuccess {
                game.notify("✦ Sauvegarde importée", "L'ancienne partie est gardée en .bak")
            }.onFailure {
                game.notify("Import impossible", it.message ?: "erreur inconnue")
            }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        game = Game(filesDir.absolutePath)
        view = CellView(this)
        view.setOnTouchListener { _, e -> onTouch(e) }
        touchSlop = ViewConfiguration.get(this).scaledTouchSlop
        val font = FontFamily(view.regular)
        setContent {
            Column(Modifier.fillMaxSize().background(Color.Black)) {
                AndroidView(factory = { view }, modifier = Modifier.weight(1f).fillMaxWidth())
                if (barVisible.value) {
                    TabBar(tabs.value, font) { game.selectTab(it) }
                }
            }
        }
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                game.back()
                if (game.shouldQuit()) finish()
            }
        })
    }

    override fun onResume() {
        super.onResume()
        if (!firstResume) {
            runCatching { game.resume() }.onFailure { Log.e(TAG, "reprise", it) }
        }
        firstResume = false
        running = true
        lastFrameNanos = 0L
        Choreographer.getInstance().postFrameCallback(this)
    }

    override fun onPause() {
        running = false
        Choreographer.getInstance().removeFrameCallback(this)
        runCatching { game.pause() }.onFailure { Log.e(TAG, "sauvegarde", it) }
        super.onPause()
    }

    override fun doFrame(frameTimeNanos: Long) {
        if (!running) return
        Choreographer.getInstance().postFrameCallback(this)
        val interval = 1_000_000_000L / game.frameRate().toLong()
        // Petite tolérance : un écran 60 Hz cadencé à 30 images/s saute une vsync sur deux.
        if (lastFrameNanos != 0L && frameTimeNanos - lastFrameNanos < interval - 2_000_000L) return
        val dt = if (lastFrameNanos == 0L) 0f else (frameTimeNanos - lastFrameNanos) / 1e9f
        lastFrameNanos = frameTimeNanos

        if (gesture == Gesture.PRESS || gesture == Gesture.CAPTURED) game.touchHold()
        applyFling(dt)
        handleRequests()

        if (view.surfaceReady) {
            val cols = if (game.wantsDense()) DENSE_COLS else GAME_COLS
            val (c, r) = view.layoutFor(cols)
            view.grid.apply(game.frame(c.toUShort(), r.toUShort()))
            view.render()
        }
        val visible = game.barVisible()
        if (visible != barVisible.value) barVisible.value = visible
        if (visible) {
            val t = game.tabs()
            if (t != tabs.value) tabs.value = t
        }
        if (game.shouldQuit()) finish()
    }

    private fun handleRequests() {
        when (game.takeRequest()) {
            Request.EXPORT_SAVE -> exportLauncher.launch("fonderie-stellaire-save.json")
            Request.IMPORT_SAVE -> importLauncher.launch(arrayOf("*/*"))
            null -> {}
        }
    }

    private fun applyFling(dt: Float) {
        if (gesture != Gesture.NONE || abs(flingRows) < 1f) {
            flingRows = 0f
            return
        }
        flingAcc += flingRows * dt
        val rows = flingAcc.toInt()
        if (rows != 0) {
            game.scrollBy(rows)
            flingAcc -= rows
        }
        flingRows *= exp(-FLING_FRICTION * dt)
    }

    private fun onTouch(e: MotionEvent): Boolean {
        if (e.actionIndex != 0 && e.actionMasked != MotionEvent.ACTION_MOVE) return true
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                flingRows = 0f
                flingAcc = 0f
                downX = e.x
                downY = e.y
                lastY = e.y
                scrollAcc = 0f
                velocity?.recycle()
                velocity = VelocityTracker.obtain().also { it.addMovement(e) }
                gesture = Gesture.PRESS
                val (col, row) = view.cellAt(e.x, e.y)
                game.touchDown(col.toUShort(), row.toUShort())
            }
            MotionEvent.ACTION_MOVE -> {
                velocity?.addMovement(e)
                if (gesture == Gesture.PRESS || gesture == Gesture.CAPTURED) {
                    val (col, row) = view.cellAt(e.x, e.y)
                    if (game.touchMove(col.toUShort(), row.toUShort())) {
                        gesture = Gesture.CAPTURED
                        return true
                    }
                }
                val dx = e.x - downX
                val dy = e.y - downY
                if (gesture == Gesture.PRESS && (abs(dx) > touchSlop || abs(dy) > touchSlop)) {
                    game.touchCancel()
                    gesture = if (abs(dy) >= abs(dx)) Gesture.SCROLL else Gesture.SWIPE
                    lastY = e.y
                }
                if (gesture == Gesture.SCROLL) {
                    // Doigt vers le haut : la liste descend.
                    scrollAcc += (lastY - e.y) / view.cellH
                    lastY = e.y
                    val rows = scrollAcc.toInt()
                    if (rows != 0) {
                        game.scrollBy(rows)
                        scrollAcc -= rows
                    }
                }
            }
            MotionEvent.ACTION_UP -> {
                velocity?.addMovement(e)
                when (gesture) {
                    Gesture.PRESS, Gesture.CAPTURED -> {
                        val (col, row) = view.cellAt(e.x, e.y)
                        game.touchUp(col.toUShort(), row.toUShort())
                    }
                    Gesture.SCROLL -> {
                        velocity?.computeCurrentVelocity(1000)
                        val vy = velocity?.yVelocity ?: 0f
                        flingRows = -vy / view.cellH
                    }
                    Gesture.SWIPE -> {
                        val dx = e.x - downX
                        if (abs(dx) > view.width * SWIPE_FRACTION) game.swipe(if (dx < 0) 1 else -1)
                    }
                    Gesture.NONE -> {}
                }
                gesture = Gesture.NONE
                velocity?.recycle()
                velocity = null
            }
            MotionEvent.ACTION_CANCEL -> {
                if (gesture == Gesture.PRESS || gesture == Gesture.CAPTURED) game.touchCancel()
                gesture = Gesture.NONE
                velocity?.recycle()
                velocity = null
            }
        }
        return true
    }

    companion object {
        private const val TAG = "Fonderie"
        /** Colonnes en portrait pour les écrans de jeu et pour les écrans denses. */
        private const val GAME_COLS = 44
        private const val DENSE_COLS = 88
        /** Part de la largeur à balayer pour changer d'onglet. */
        private const val SWIPE_FRACTION = 0.2f
        /** Freinage de l'inertie (par seconde). */
        private const val FLING_FRICTION = 3.5f
    }
}
