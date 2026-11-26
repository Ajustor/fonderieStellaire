//! Composition root : instancie les adapters d'infrastructure, les injecte dans
//! le `GameService`, puis lance la présentation.

use std::error::Error;
use std::process::ExitCode;

use application::GameService;
use infrastructure::{JsonSaveRepository, SystemClock, XorShift64};

const USAGE: &str = "\
Fonderie Stellaire — un idle game cosmique dans ton terminal

Usage : fonderie-stellaire [--reset]

  --reset   repartir de zéro (l'ancienne sauvegarde est renommée en .bak)
  --help    afficher cette aide";

fn run() -> Result<(), Box<dyn Error>> {
    let mut reset = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--reset" => reset = true,
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

    let mut service = GameService::new(
        Box::new(repo),
        Box::new(SystemClock),
        Box::new(XorShift64::from_time()),
    );
    let startup = service.load()?;
    let outcome = presentation::run(service, startup)?;
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
