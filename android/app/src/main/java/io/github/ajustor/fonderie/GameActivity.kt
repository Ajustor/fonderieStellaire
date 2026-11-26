package io.github.ajustor.fonderie

import android.app.Activity
import android.os.Bundle
import android.util.Log
import android.view.Choreographer
import android.view.MotionEvent
import android.view.View
import android.window.OnBackInvokedDispatcher
import android.os.Build
import uniffi.fonderie.Action
import uniffi.fonderie.Game

/**
 * Cycle de vie et boucle d'images : à chaque image, Rust avance le jeu et
 * renvoie les cellules modifiées, que `CellView` dessine.
 */
class GameActivity : Activity(), Choreographer.FrameCallback {
    private lateinit var game: Game
    private lateinit var view: CellView

    private var running = false
    private var lastFrameNanos = 0L
    private var firstResume = true

    /** Doigt posé : instant (ns) et dernière répétition envoyée. */
    private var touchDownAt = 0L
    private var lastHoldAt = 0L
    private var touching = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        game = Game(filesDir.absolutePath)
        view = CellView(this)
        view.setOnTouchListener { _, e -> onTouch(e) }
        view.systemUiVisibility = View.SYSTEM_UI_FLAG_LAYOUT_STABLE
        setContentView(view)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            onBackInvokedDispatcher.registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_DEFAULT,
            ) { onBack() }
        }
    }

    @Deprecated("Remplacé par OnBackInvokedDispatcher à partir d'Android 13")
    override fun onBackPressed() {
        onBack()
    }

    private fun onBack() {
        game.action(Action.BACK)
        if (game.shouldQuit()) finish()
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
        lastFrameNanos = frameTimeNanos

        if (touching && frameTimeNanos - touchDownAt > HOLD_DELAY &&
            frameTimeNanos - lastHoldAt > HOLD_INTERVAL
        ) {
            lastHoldAt = frameTimeNanos
            game.hold()
        }
        if (!view.surfaceReady) return
        val cols = if (game.wantsDense()) DENSE_COLS else GAME_COLS
        val (c, r) = view.layoutFor(cols)
        view.grid.apply(game.frame(c.toUShort(), r.toUShort()))
        view.render()
        if (game.shouldQuit()) finish()
    }

    private fun onTouch(e: MotionEvent): Boolean {
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                val (col, row) = view.cellAt(e.x, e.y)
                touching = true
                touchDownAt = System.nanoTime()
                lastHoldAt = touchDownAt
                game.press(col.toUShort(), row.toUShort())
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                touching = false
                game.release()
            }
        }
        return true
    }

    companion object {
        private const val TAG = "Fonderie"
        /** Colonnes en portrait pour les écrans de jeu et pour le lancement. */
        private const val GAME_COLS = 44
        private const val DENSE_COLS = 88
        /** Maintien : délai avant la récolte continue, puis cadence des répétitions. */
        private const val HOLD_DELAY = 350_000_000L
        private const val HOLD_INTERVAL = 33_000_000L
    }
}
