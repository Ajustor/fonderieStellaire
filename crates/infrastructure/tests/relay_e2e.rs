//! Aller-retour réel avec le relais : `FONDERIE_RELAY_TEST=http://127.0.0.1:8080
//! cargo test -p infrastructure --test relay_e2e -- --ignored`.
#![cfg(feature = "sync")]

use application::sync::{reconcile, Reconciled};
use application::{SaveData, SyncBackend};
use infrastructure::RelaySync;

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fonderie-e2e-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
#[ignore = "demande un relais lancé (FONDERIE_RELAY_TEST)"]
fn two_devices_through_a_real_relay() {
    let url = std::env::var("FONDERIE_RELAY_TEST").expect("FONDERIE_RELAY_TEST");
    let mut pc = RelaySync::new(&url, &dir("pc"));
    let code = pc.create_link().unwrap();
    let played = SaveData {
        version: application::SAVE_VERSION,
        play_time: 120.0,
        total_all: 5e6,
        stardust: 42.0,
        ..SaveData::default()
    };
    assert_eq!(reconcile(&mut pc, &played), Ok(Reconciled::Pushed));

    let mut phone = RelaySync::new(&url, &dir("phone"));
    phone.join_link(&code.to_lowercase()).unwrap();
    let fresh = SaveData {
        version: application::SAVE_VERSION,
        play_time: 3.0,
        ..SaveData::default()
    };
    let Ok(Reconciled::Adopt(remote)) = reconcile(&mut phone, &fresh) else {
        panic!("le téléphone doit reprendre la partie du PC");
    };
    assert_eq!(remote.data.stardust, 42.0);
    assert_eq!(remote.data.play_time, 120.0);
    phone.set_synced_rev(remote.rev).unwrap();
    let more = SaveData {
        play_time: 200.0,
        ..remote.data.clone()
    };
    assert_eq!(reconcile(&mut phone, &more), Ok(Reconciled::Pushed));
    assert!(matches!(
        reconcile(&mut pc, &played),
        Ok(Reconciled::Adopt(_))
    ));
}

#[test]
#[ignore = "demande un relais lancé (FONDERIE_RELAY_TEST)"]
fn leaderboard_through_a_real_relay() {
    use application::{Board, LbError, LeaderboardBackend, ScoreCard};
    use infrastructure::RelayLeaderboard;

    let url = std::env::var("FONDERIE_RELAY_TEST").expect("FONDERIE_RELAY_TEST");
    let (pc_dir, phone_dir) = (dir("lb-pc"), dir("lb-phone"));
    let mut pc_lb = RelayLeaderboard::new(&url, &pc_dir);
    assert!(matches!(pc_lb.claim("Nova"), Err(LbError::NotLinked)));
    let code = RelaySync::new(&url, &pc_dir).create_link().unwrap();
    // Pseudo unique par lien : un nom aléatoire évite les collisions entre essais.
    let pseudo = format!("e2e-{}", &code[..5]);
    assert_eq!(pc_lb.claim(&pseudo), Ok(pseudo.clone()));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let card = ScoreCard {
        act: 1,
        progress: 0.3,
        total_log: 8.0,
        supernovas: 1,
        crunches: 0,
        big_bangs: 0,
        achievements: 5,
        play_time: 1800.0,
        daily: Some((now / 86_400, 123_456.0)),
        platform: "terminal",
        version: "e2e".into(),
        sent_at: now,
    };
    pc_lb.submit(&card).unwrap();

    // Le téléphone rejoint le lien : même joueur, pseudo relu depuis le relais.
    RelaySync::new(&url, &phone_dir).join_link(&code).unwrap();
    let mut phone_lb = RelayLeaderboard::new(&url, &phone_dir);
    assert_eq!(phone_lb.pseudo(), None);
    assert_eq!(phone_lb.refresh_pseudo(), Ok(Some(pseudo.clone())));
    assert_eq!(phone_lb.pseudo(), Some(pseudo.clone()));

    let general = phone_lb.standings(Board::General, now / 86_400).unwrap();
    let me = general.me.expect("ma ligne");
    assert_eq!((me.pseudo.as_str(), me.act), (pseudo.as_str(), 1));
    let daily = phone_lb.standings(Board::Daily, now / 86_400).unwrap();
    assert_eq!(daily.me.and_then(|e| e.score), Some(123_456.0));

    phone_lb.withdraw().unwrap();
    assert_eq!(phone_lb.pseudo(), None);
    assert_eq!(pc_lb.refresh_pseudo(), Ok(None));
}
