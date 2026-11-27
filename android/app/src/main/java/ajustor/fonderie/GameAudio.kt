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
 * Arrêtée en arrière-plan ; muette en mode silencieux ou vibreur.
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
        var lastRingerCheck = 0L
        try {
            track.play()
            while (running) {
                // Mode silencieux ou vibreur : le jeu se tait.
                val now = SystemClock.elapsedRealtime()
                if (now - lastRingerCheck > RINGER_CHECK_MS) {
                    lastRingerCheck = now
                    game.setAudioMuted(audioManager.ringerMode != AudioManager.RINGER_MODE_NORMAL)
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
        private const val RINGER_CHECK_MS = 500L
    }
}
