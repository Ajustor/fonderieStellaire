//! Composition root : instancie les adapters d'infrastructure, les injecte dans
//! le `GameService`, puis lance la présentation.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use application::simulator::{self, fmt_play_time, SimConfig};
use application::{GameService, SyncBackend, SyncService, UpdateService};
use infrastructure::{GitHubUpdater, JsonSaveRepository, RelaySync, SystemClock, XorShift64};

/// Dépôt GitHub où sont publiées les versions (mise à jour automatique).
const REPO: &str = "Ajustor/fonderieStellaire";

const USAGE: &str = "\
Fonderie Stellaire — un idle game cosmique dans ton terminal

Usage : fonderie-stellaire [option]

  --reset              repartir de zéro (l'ancienne sauvegarde est renommée en .bak)
  --export <fichier>   copier ta sauvegarde dans un fichier (pour la garder ou
                       la transférer sur un autre ordinateur)
  --import <fichier>   reprendre une sauvegarde exportée (l'actuelle passe en .bak)
  --link <code>        lier cet ordinateur à un autre appareil (code affiché dans
                       Options › Synchronisation) : la même partie partout,
                       la plus jouée est gardée
  --unlink             ne plus synchroniser cet ordinateur
  --simulate [h/jour]  simuler une partie complète et afficher le rythme :
                       en jeu continu, ou avec N heures de jeu par jour
                       (le reste du temps, le jeu est fermé)
  --version            afficher la version
  --help               afficher cette aide

Le jeu se met à jour tout seul au lancement.";

/// Version du jeu : le tag de la release (injecté par la CI) ou celle de Cargo.
fn version() -> &'static str {
    match option_env!("FONDERIE_VERSION") {
        Some(v) if !v.trim().is_empty() => v.trim().trim_start_matches('v'),
        _ => env!("CARGO_PKG_VERSION"),
    }
}

/// Relance le jeu (nouvelle version installée) et attend qu'il se termine pour
/// rendre la main au terminal.
fn relaunch(exe: &Path) -> Result<(), Box<dyn Error>> {
    let status = std::process::Command::new(exe)
        .status()
        .map_err(|e| format!("redémarrage impossible ({e}) : relance le jeu à la main"))?;
    if status.success() {
        Ok(())
    } else {
        Err("la nouvelle version s'est arrêtée sur une erreur".into())
    }
}

/// `--export` : copie la sauvegarde actuelle (après vérification) dans `dest`.
fn export(repo: &JsonSaveRepository, dest: &Path) -> Result<(), Box<dyn Error>> {
    repo.export_to(dest)?;
    println!("✦ Sauvegarde exportée dans {}", dest.display());
    Ok(())
}

/// `--import` : vérifie le fichier puis le met à la place de la sauvegarde.
fn import(repo: &JsonSaveRepository, src: &Path) -> Result<(), Box<dyn Error>> {
    if repo.import_from(src)? {
        println!(
            "Sauvegarde actuelle mise de côté : {}",
            repo.backup_path().display()
        );
    }
    println!("✦ Sauvegarde importée : lance le jeu pour la reprendre.");
    Ok(())
}

/// Synchronisation : le lien est gardé à côté de la sauvegarde.
fn relay(repo: &JsonSaveRepository) -> Option<RelaySync> {
    let dir = repo.path().parent()?;
    RelaySync::from_env(dir)
}

/// `--link` / `--unlink` : enregistre ou oublie le lien ; la synchronisation
/// a lieu au prochain lancement.
fn link(repo: &JsonSaveRepository, code: Option<&str>) -> Result<(), Box<dyn Error>> {
    let mut sync = relay(repo)
        .ok_or("synchronisation indisponible dans cette version (relais non configuré)")?;
    match code {
        Some(code) => {
            let code = sync.join_link(code)?;
            println!("✦ Ordinateur lié ({code}).");
            println!(
                "  Lance le jeu : la partie la plus jouée sera gardée sur tous les appareils."
            );
        }
        None => {
            sync.unlink()?;
            println!("✦ Cet ordinateur n'est plus synchronisé. Ta partie reste ici.");
        }
    }
    Ok(())
}

/// `--simulate` : une partie entière jouée par une stratégie automatique.
fn simulate(daily_hours: Option<f64>) {
    let catalog = domain::Catalog::standard();
    let cfg = SimConfig {
        daily_hours,
        max_secs: if daily_hours.is_some() { 180.0 } else { 60.0 } * 86_400.0,
        ..SimConfig::default()
    };
    match daily_hours {
        Some(h) => println!(
            "Simulation d'une partie : {h} h de jeu par jour, jeu fermé le reste du temps (durées en jours réels)…"
        ),
        None => println!("Simulation d'une partie en jeu continu (1 clic/s)…"),
    }
    let report = simulator::simulate(&catalog, &cfg, |t| {
        if ((t / 3600.0) as u64).is_multiple_of(24) {
            eprint!("\r  {} simulés…   ", fmt_play_time(t));
        }
    });
    eprintln!();
    println!();
    for m in &report.milestones {
        println!("  {:<42} {:>12}", m.label, fmt_play_time(m.at));
    }
    println!();
    println!(
        "  {} Supernovas · {} Big Crunch au total",
        report.supernovas_total, report.crunches_total
    );
    if report.finished {
        println!(
            "  Tout le contenu terminé en {}.",
            fmt_play_time(report.simulated)
        );
    } else {
        println!(
            "  Contenu non terminé après {}.",
            fmt_play_time(report.simulated)
        );
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // Chemin de l'exécutable au lancement : la mise à jour remplace le fichier
    // à cet endroit, c'est donc lui qu'on relance.
    let exe = std::env::current_exe().ok();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut reset = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--reset" => reset = true,
            "--export" | "--import" => {
                let file = it
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| format!("{arg} attend un nom de fichier\n\n{USAGE}"))?;
                let repo = JsonSaveRepository::default_location()?;
                return if arg == "--export" {
                    export(&repo, &file)
                } else {
                    import(&repo, &file)
                };
            }
            "--link" => {
                let code = it
                    .next()
                    .ok_or_else(|| format!("--link attend le code de lien\n\n{USAGE}"))?;
                return link(&JsonSaveRepository::default_location()?, Some(code));
            }
            "--unlink" => return link(&JsonSaveRepository::default_location()?, None),
            "--simulate" => {
                let hours = it.next().and_then(|d| d.parse::<f64>().ok());
                simulate(hours.map(|h| h.clamp(0.5, 24.0)));
                return Ok(());
            }
            "-V" | "--version" => {
                println!("fonderie-stellaire {}", version());
                return Ok(());
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("option inconnue : {other}\n\n{USAGE}").into()),
        }
    }

    let repo = JsonSaveRepository::default_location()?;
    if reset && repo.reset()? {
        println!(
            "Ancienne sauvegarde mise de côté : {}",
            repo.backup_path().display()
        );
    }

    let updater = GitHubUpdater::new(REPO);
    updater.cleanup_previous();
    let updates = UpdateService::spawn(Box::new(updater), version());

    let sync = relay(&repo)
        .map(|r| SyncService::spawn(Box::new(r)))
        .unwrap_or_else(SyncService::disabled);

    let mut service = GameService::new(
        Box::new(repo),
        Box::new(SystemClock),
        Box::new(XorShift64::from_time()),
    );
    let startup = service.load()?;
    let outcome = presentation::run(service, startup, updates, sync)?;
    if let Some(e) = outcome.save_error {
        return Err(format!("la sauvegarde finale a échoué : {e}").into());
    }
    if outcome.restart {
        if let Some(exe) = exe {
            return relaunch(&exe);
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Fonderie Stellaire : {e}");
            ExitCode::FAILURE
        }
    }
}
