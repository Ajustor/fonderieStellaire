package ajustor.fonderie

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.os.Process
import android.os.SystemClock
import android.util.Log
import uniffi.fonderie.Game

/**
 * Sortie des sons du jeu : un fil qui tire le PCM mélangé par Rust
 * (`Game.audioFill`) et l'écrit dans un AudioTrack en faible latence.
 * Arrêtée en arrière-plan, en pause dès que plus rien ne joue. Comme dans les autres jeux, le volume est celui
 * du multimédia : le mode sonnerie (vibreur, silencieux) n'y change rien.
 */
class GameAudio(context: Context, private val game: Game) {
    private val audioManager = context.getSystemService(AudioManager::class.java)
    private var thread: Thread? = null

    @Volatile
    private var running = false

    fun start() {
        if (running) return
        running = true
        thread = Thread(::loop, "fonderie-audio").also { it.start() }
    }

    fun stop() {
        running = false
        thread?.join(500)
        thread = null
    }

    private fun loop() {
        Process.setThreadPriority(Process.THREAD_PRIORITY_URGENT_AUDIO)
        val rate = audioManager.getProperty(AudioManager.PROPERTY_OUTPUT_SAMPLE_RATE)
            ?.toIntOrNull() ?: 48_000
        val burst = audioManager.getProperty(AudioManager.PROPERTY_OUTPUT_FRAMES_PER_BUFFER)
            ?.toIntOrNull()?.coerceIn(64, 2048) ?: 256
        game.audioStart(rate.toUInt())
        val track = try {
            val min = AudioTrack.getMinBufferSize(
                rate,
                AudioFormat.CHANNEL_OUT_MONO,
                AudioFormat.ENCODING_PCM_16BIT,
            )
            AudioTrack.Builder()
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_GAME)
                        .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                        .build(),
                )
                .setAudioFormat(
                    AudioFormat.Builder()
                        .setSampleRate(rate)
                        .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                        .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                        .build(),
                )
                .setBufferSizeInBytes(maxOf(min, burst * 2 * 2))
                .setTransferMode(AudioTrack.MODE_STREAM)
                .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                .build()
        } catch (e: Exception) {
            Log.w(TAG, "sortie audio indisponible", e)
            running = false
            return
        }
        try {
            track.play()
            var playing = true
            var silentSince = SystemClock.elapsedRealtime()
            while (running) {
                val now = SystemClock.elapsedRealtime()
                if (game.audioActive()) {
                    silentSince = now
                    if (!playing) {
                        track.play()
                        playing = true
                    }
                } else if (playing && now - silentSince >= IDLE_MS) {
                    // Rien ne joue : sortie en pause, le circuit audio se met
                    // en veille au lieu de recevoir du silence en continu.
                    track.pause()
                    track.flush()
                    playing = false
                }
                if (!playing) {
                    Thread.sleep(POLL_MS)
                    continue
                }
                val pcm = game.audioFill(burst.toUInt())
                // Écriture bloquante : elle cadence la boucle sur la sortie.
                if (track.write(pcm, 0, pcm.size) < 0) break
            }
        } catch (e: Exception) {
            Log.w(TAG, "sortie audio interrompue", e)
        } finally {
            runCatching { track.stop() }
            track.release()
        }
    }

    companion object {
        private const val TAG = "FonderieAudio"
        /** Silence au bout duquel la sortie est mise en pause. */
        private const val IDLE_MS = 2_000L
        /** Attente entre deux vérifications pendant la pause (latence de reprise). */
        private const val POLL_MS = 15L
    }
}
