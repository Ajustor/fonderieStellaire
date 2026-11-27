package ajustor.fonderie

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.net.Uri
import android.os.Build
import uniffi.fonderie.Game

/**
 * Avis de nouvelle version pour l'app installée par l'APK de GitHub : au
 * lancement, le jeu cherche une version plus récente (`Game.checkUpdates`) ;
 * s'il en trouve une, il demande `Request.UPDATE_AVAILABLE` et cette fenêtre
 * propose de télécharger l'APK (le navigateur la télécharge, Android demande
 * ensuite la confirmation d'installation ; la partie est conservée).
 *
 * Jamais pour une app installée depuis le Play Store, qui gère ses mises à
 * jour. **Pour retirer le système** une fois l'app sur le Play Store :
 * `apkUpdates=false` dans `android/gradle.properties` (plus aucune
 * vérification) ; ou supprimer ce fichier et ses appels dans `GameActivity`
 * (`apkUpdates.start()`, `apkUpdates.show()`).
 */
class ApkUpdates(private val activity: Activity, private val game: Game) {
    private val dialogTheme = android.R.style.Theme_DeviceDefault_Dialog_Alert
    private var shown = false

    /** À l'ouverture de l'app : vérification en arrière-plan, si activée. */
    fun start() {
        if (BuildConfig.APK_UPDATES && !installedFromPlayStore()) game.checkUpdates()
    }

    /**
     * Fenêtre de la nouvelle version. `auto` : affichée d'elle-même au
     * lancement (une seule fois) ; sinon demandée par le joueur (*Plus ›
     * Sauver*, ligne de la version).
     */
    fun show(auto: Boolean) {
        if (auto && shown) return
        val info = game.updateInfo() ?: return
        shown = true
        val builder = AlertDialog.Builder(activity, dialogTheme)
            .setTitle("Nouvelle version ${info.version}")
            .setMessage(
                "Télécharge l'APK, puis ouvre-la pour l'installer : ta partie est conservée.\n\n" +
                    "Android te demandera peut-être d'autoriser l'installation depuis ton navigateur.",
            )
            .setNegativeButton("Plus tard", null)
            .setNeutralButton("Nouveautés") { _, _ -> open(info.page) }
        val apk = info.apk
        if (apk != null) {
            builder.setPositiveButton("Télécharger") { _, _ -> open(apk) }
        } else {
            builder.setPositiveButton("Ouvrir la page") { _, _ -> open(info.page) }
        }
        builder.show()
    }

    private fun open(url: String) {
        runCatching { activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }
            .onFailure { game.notify("Aucun navigateur", url) }
    }

    /** Installée depuis le Play Store (qui gère alors les mises à jour). */
    private fun installedFromPlayStore(): Boolean = runCatching {
        val pm = activity.packageManager
        val name = activity.packageName
        val installer = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            pm.getInstallSourceInfo(name).installingPackageName
        } else {
            @Suppress("DEPRECATION")
            pm.getInstallerPackageName(name)
        }
        installer == "com.android.vending"
    }.getOrDefault(false)
}
