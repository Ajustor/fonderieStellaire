//! Acte IV : le Défi du jour. Un sprint de 10 minutes dans un univers de poche
//! (une partie vierge) avec deux règles tirées de la date : les mêmes pour
//! tous les joueurs ce jour-là. Le score est la poussière récoltée ; un code
//! de score permet de le partager.

use crate::universe::UniverseRules;

/// Durée d'un défi (secondes de jeu).
pub const DAILY_SECS: f64 = 10.0 * 60.0;

/// Une règle possible du jour : son texte et ce qu'elle change.
struct Modifier {
    text: &'static str,
    apply: fn(&mut UniverseRules),
}

const MODIFIERS: [Modifier; 8] = [
    Modifier {
        text: "Production ×3, générateurs ×2 plus chers",
        apply: |r| {
            r.production *= 3.0;
            r.cost *= 2.0;
        },
    },
    Modifier {
        text: "Comètes 5 fois plus fréquentes",
        apply: |r| r.comet_interval *= 0.2,
    },
    Modifier {
        text: "Aucune amélioration, production ×5",
        apply: |r| {
            r.no_upgrades = true;
            r.production *= 5.0;
        },
    },
    Modifier {
        text: "Entropie : −3 %/min, production ×4",
        apply: |r| {
            r.entropy_per_min += 0.03;
            r.production *= 4.0;
        },
    },
    Modifier {
        text: "Tempêtes solaires 3 fois plus longues",
        apply: |r| r.storm_duration *= 3.0,
    },
    Modifier {
        text: "Générateurs à moitié prix",
        apply: |r| r.cost *= 0.5,
    },
    Modifier {
        text: "Production ×0,5, comètes 3 fois plus fréquentes",
        apply: |r| {
            r.production *= 0.5;
            r.comet_interval *= 0.33;
        },
    },
    Modifier {
        text: "Production ×2",
        apply: |r| r.production *= 2.0,
    },
];

/// Hasard déterministe du jour.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Les deux règles du jour `day` (jours depuis 1970) : (textes, règles).
pub fn rules_for(day: u64) -> (Vec<&'static str>, UniverseRules) {
    let a = (mix(day) % MODIFIERS.len() as u64) as usize;
    let mut b = (mix(day ^ 0xDA11) % (MODIFIERS.len() as u64 - 1)) as usize;
    if b >= a {
        b += 1;
    }
    let mut rules = UniverseRules::NORMAL;
    (MODIFIERS[a].apply)(&mut rules);
    (MODIFIERS[b].apply)(&mut rules);
    (vec![MODIFIERS[a].text, MODIFIERS[b].text], rules)
}

/// Matière exotique gagnée pour un score (première partie du jour).
pub fn reward_for(score: f64) -> f64 {
    (score.max(1.0).log10() - 4.0).clamp(0.0, 12.0)
}

/// Code de score à partager : `FS<jour>-<log10 du score>-<contrôle>`.
pub fn share_code(day: u64, score: f64) -> String {
    let log = (score.max(1.0).log10() * 100.0).round() as u64;
    let check = (mix(day.wrapping_mul(31) ^ log ^ 0x5EED) & 0xFFFF) as u16;
    format!("FS{day}-{}.{:02}-{check:04X}", log / 100, log % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_day_same_rules_and_two_different_modifiers() {
        let (a, ra) = rules_for(20_000);
        let (b, rb) = rules_for(20_000);
        assert_eq!(a, b);
        assert_eq!(ra, rb);
        assert_eq!(a.len(), 2);
        assert_ne!(a[0], a[1]);
        let differs = (20_001..20_010).any(|d| rules_for(d).0 != a);
        assert!(differs, "les jours changent les règles");
    }

    #[test]
    fn reward_and_code() {
        assert_eq!(reward_for(1e3), 0.0);
        assert_eq!(reward_for(1e10), 6.0);
        assert_eq!(reward_for(1e30), 12.0);
        let code = share_code(20_000, 1.5e9);
        assert!(code.starts_with("FS20000-9.18-"), "{code}");
        assert_eq!(code, share_code(20_000, 1.5e9));
        assert_ne!(code, share_code(20_000, 1.6e9));
    }
}
