//! Composition root : instancie les adapters d'infrastructure, les injecte dans
//! le `GameService`, puis lance la présentation.

use std::error::Error;
use std::process::ExitCode;

use application::{GameService, UpdateService};
use infrastructure::{GitHubUpdater, JsonSaveRepository, SystemClock, XorShift64};

/// Dépôt GitHub où sont publiées les versions (mise à jour automatique).
const REPO: &str = "Ajustor/fonderieStellaire";

const USAGE: &str = "\
Fonderie Stellaire — un idle game cosmique dans ton terminal

Usage : fonderie-stellaire [--reset] [--no-update]

  --reset       repartir de zéro (l'ancienne sauvegarde est renommée en .bak)
  --no-update   ne pas chercher de mise à jour (ou FONDERIE_NO_UPDATE=1)
  --version     afficher la version
  --help        afficher cette aide";

/// Version du jeu : le tag de la release (injecté par la CI) ou celle de Cargo.
fn version() -> &'static str {
    match option_env!("FONDERIE_VERSION") {
        Some(v) if !v.trim().is_empty() => v.trim().trim_start_matches('v'),
        _ => env!("CARGO_PKG_VERSION"),
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut reset = false;
    let mut no_update = std::env::var("FONDERIE_NO_UPDATE")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false);
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--reset" => reset = true,
            "--no-update" => no_update = true,
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

    let updates = if no_update {
        UpdateService::disabled(version())
    } else {
        let updater = GitHubUpdater::new(REPO);
        updater.cleanup_previous();
        UpdateService::spawn(Box::new(updater), version())
    };

    let mut service = GameService::new(
        Box::new(repo),
        Box::new(SystemClock),
        Box::new(XorShift64::from_time()),
    );
    let startup = service.load()?;
    let outcome = presentation::run(service, startup, updates)?;
    if let Some(e) = outcome.save_error {
        return Err(format!("la sauvegarde finale a échoué : {e}").into());
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
