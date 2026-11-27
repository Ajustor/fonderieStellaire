package ajustor.fonderie

import android.app.Activity
import android.app.AlertDialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.Typeface
import android.os.Handler
import android.os.Looper
import android.text.InputFilter
import android.text.InputType
import android.view.Gravity
import android.widget.EditText
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter
import uniffi.fonderie.Game
import uniffi.fonderie.SyncInfo

/**
 * Fenêtres natives de la synchronisation entre appareils : QR code et code du
 * lien, scan d'un QR code (Google Code Scanner : aucune autorisation caméra
 * pour l'app), saisie du code à la main. Le réseau et les décisions sont
 * dans Rust (`Game.sync*`).
 */
class SyncDialogs(private val activity: Activity, private val game: Game) {
    private val main = Handler(Looper.getMainLooper())
    private val dialogTheme = android.R.style.Theme_DeviceDefault_Dialog_Alert

    fun show() {
        val info = game.syncInfo()
        when {
            !info.available -> AlertDialog.Builder(activity, dialogTheme)
                .setTitle("Synchronisation")
                .setMessage("Indisponible dans cette version.")
                .setPositiveButton("Fermer", null)
                .show()
            info.code == null -> showUnlinked()
            else -> showLinked(info)
        }
    }

    private fun showUnlinked() {
        AlertDialog.Builder(activity, dialogTheme)
            .setTitle("Synchroniser entre appareils")
            .setMessage(
                "Joue sur ton ordinateur et ton téléphone avec la même partie, sans compte.\n\n" +
                    "• Sur l'autre appareil, ouvre la synchronisation (Options › Synchronisation sur " +
                    "ordinateur) et scanne son QR code, ou tape son code.\n" +
                    "• Ou crée un lien ici et scanne-le depuis l'autre téléphone.\n\n" +
                    "La partie la plus jouée est gardée ; l'autre est mise de côté en .bak.",
            )
            .setPositiveButton("Scanner") { _, _ -> scan() }
            .setNeutralButton("Taper un code") { _, _ -> typeCode() }
            .setNegativeButton("Créer un lien") { _, _ -> create() }
            .show()
    }

    private fun showLinked(info: SyncInfo) {
        val pad = dp(20)
        val layout = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(pad, pad / 2, pad, 0)
        }
        info.qrText?.let { text ->
            val size = dp(220)
            layout.addView(
                ImageView(activity).apply { setImageBitmap(qrBitmap(text, size)) },
                LinearLayout.LayoutParams(size, size),
            )
        }
        layout.addView(TextView(activity).apply {
            this.text = info.code
            typeface = Typeface.MONOSPACE
            textSize = 18f
            setTextIsSelectable(true)
            gravity = Gravity.CENTER
            setPadding(0, pad / 2, 0, pad / 4)
        })
        layout.addView(TextView(activity).apply {
            this.text = "Scanne ce code depuis l'autre téléphone, ou sur ordinateur :\n" +
                "fonderie-stellaire --link ${info.code}\n\n${info.status}"
            textSize = 13f
            gravity = Gravity.CENTER
        })
        val pseudo = game.leaderboardInfo().pseudo
        val pseudoLine = TextView(activity).apply {
            this.text = if (pseudo != null) "★ Classement : $pseudo (toucher pour changer)"
            else "★ Choisir un pseudo pour le classement (facultatif)"
            textSize = 14f
            gravity = Gravity.CENTER
            setTextColor(Color.rgb(245, 200, 76))
            setPadding(0, pad / 2, 0, 0)
        }
        layout.addView(pseudoLine)
        val dialog = AlertDialog.Builder(activity, dialogTheme)
            .setTitle("Appareil lié")
            .setView(layout)
            .setPositiveButton("Fermer") { _, _ -> game.syncNow() }
            .setNeutralButton("Copier le code") { _, _ -> copy(info.code ?: "") }
            .setNegativeButton("Délier") { _, _ -> confirmUnlink() }
            .show()
        // La ligne du pseudo ouvre sa saisie.
        pseudoLine.setOnClickListener {
            dialog.dismiss()
            askPseudo()
        }
    }

    /**
     * Pseudo du classement : facultatif, il faut un appareil synchronisé. Le
     * réseau et la validation sont dans Rust (`Game.setPseudo`).
     */
    fun askPseudo() {
        val info = game.leaderboardInfo()
        if (!info.available) {
            game.notify("Classement", "Indisponible dans cette version")
            return
        }
        if (!info.linked) {
            AlertDialog.Builder(activity, dialogTheme)
                .setTitle("Classement")
                .setMessage("Pour apparaître au classement, synchronise d'abord ta partie, puis choisis un pseudo.")
                .setPositiveButton("Synchroniser") { _, _ -> show() }
                .setNegativeButton("Annuler", null)
                .show()
            return
        }
        val input = EditText(activity).apply {
            hint = "3 à 16 caractères"
            setText(info.pseudo ?: "")
            setSelection(text.length)
            filters = arrayOf(InputFilter.LengthFilter(PSEUDO_MAX))
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
        }
        val pad = dp(20)
        val box = LinearLayout(activity).apply {
            setPadding(pad, pad / 2, pad, 0)
            addView(input, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT))
        }
        val builder = AlertDialog.Builder(activity, dialogTheme)
            .setTitle("Pseudo du classement")
            .setMessage("Il s'affiche avec ta progression et ton score du Défi du jour, sur le site et dans le jeu.")
            .setView(box)
            .setPositiveButton("Enregistrer") { _, _ -> game.setPseudo(input.text.toString()) }
            .setNegativeButton("Annuler", null)
        if (info.pseudo != null) {
            builder.setNeutralButton("Quitter le classement") { _, _ -> game.setPseudo("") }
        }
        builder.show()
    }

    private fun confirmUnlink() {
        AlertDialog.Builder(activity, dialogTheme)
            .setTitle("Délier cet appareil ?")
            .setMessage("Ta partie reste sur ce téléphone, mais elle ne sera plus synchronisée.")
            .setPositiveButton("Délier") { _, _ -> game.syncUnlink() }
            .setNegativeButton("Annuler", null)
            .show()
    }

    private fun create() {
        game.syncCreate()
        // Le code arrive dès que le relais a répondu.
        waitForLink(System.currentTimeMillis() + LINK_TIMEOUT_MS)
    }

    private fun waitForLink(deadline: Long) {
        main.postDelayed({
            val info = game.syncInfo()
            when {
                info.code != null && !info.busy -> showLinked(info)
                info.error || System.currentTimeMillis() > deadline -> {} // toast affiché par le jeu
                else -> waitForLink(deadline)
            }
        }, POLL_MS)
    }

    private fun scan() {
        val options = GmsBarcodeScannerOptions.Builder()
            .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
            .build()
        GmsBarcodeScanning.getClient(activity, options).startScan()
            .addOnSuccessListener { join(it.rawValue ?: "") }
            .addOnFailureListener {
                game.notify("Scan impossible", it.message ?: "tape le code à la place")
            }
    }

    private fun typeCode() {
        val input = EditText(activity).apply {
            hint = "XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS or
                InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
            typeface = Typeface.MONOSPACE
        }
        val pad = dp(20)
        val box = LinearLayout(activity).apply {
            setPadding(pad, pad / 2, pad, 0)
            addView(input, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT))
        }
        AlertDialog.Builder(activity, dialogTheme)
            .setTitle("Code de lien")
            .setMessage("Le code affiché sur l'autre appareil (25 caractères).")
            .setView(box)
            .setPositiveButton("Lier") { _, _ -> join(input.text.toString()) }
            .setNegativeButton("Annuler", null)
            .show()
    }

    private fun join(code: String) {
        runCatching { game.syncJoin(code) }
            .onFailure { game.notify("Code invalide", it.message ?: "vérifie les 25 caractères") }
    }

    private fun copy(code: String) {
        val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("Code de lien", code))
    }

    private fun qrBitmap(text: String, size: Int): Bitmap {
        val hints = mapOf(EncodeHintType.MARGIN to 2)
        val matrix = QRCodeWriter().encode(text, BarcodeFormat.QR_CODE, size, size, hints)
        val pixels = IntArray(size * size) { i ->
            if (matrix.get(i % size, i / size)) Color.BLACK else Color.WHITE
        }
        return Bitmap.createBitmap(pixels, size, size, Bitmap.Config.ARGB_8888)
    }

    private fun dp(v: Int): Int = (v * activity.resources.displayMetrics.density).toInt()

    companion object {
        private const val POLL_MS = 300L
        private const val LINK_TIMEOUT_MS = 20_000L
        private const val PSEUDO_MAX = 16
    }
}
