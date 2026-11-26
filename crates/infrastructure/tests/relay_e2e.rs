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
