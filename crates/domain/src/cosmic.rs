//! Acte III : les Événements cosmiques. De temps en temps, un dilemme se
//! présente ; chaque choix a ses conséquences, parfois risquées, souvent
//! temporaires. Et la Convergence, le grand final de l'Acte III.

/// Conséquence d'un choix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventEffect {
    Nothing,
    /// Production multipliée pendant un temps.
    Production {
        factor: f64,
        hours: f64,
    },
    /// Fragments multipliés pendant un temps.
    Fragments {
        factor: f64,
        hours: f64,
    },
    /// Des heures de production d'un coup.
    Stardust {
        hours: f64,
    },
    /// Une relique au hasard (rareté selon `danger`).
    Relic {
        danger: u8,
    },
    /// Les flottes en route parcourent d'un coup une part de leur trajet.
    FleetHaste(f64),
    /// Un Éclat (+1 % de production).
    Shard,
    /// Une part de la poussière actuelle est perdue.
    LoseStardust(f64),
    /// Une part de la poussière contre une relique rare.
    BuyRelic(f64),
}

/// Un choix : son libellé, son effet, et s'il est risqué, sa chance de
/// réussir et ce qui arrive sinon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventChoice {
    pub label: &'static str,
    pub effect: EventEffect,
    pub risk: Option<(f64, EventEffect)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CosmicEventDef {
    pub index: usize,
    pub key: &'static str,
    pub title: &'static str,
    pub text: &'static str,
    pub choices: Vec<EventChoice>,
}

/// Bonus temporaire en cours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoostKind {
    Production,
    Fragments,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boost {
    pub kind: BoostKind,
    pub factor: f64,
    /// Secondes restantes.
    pub left: f64,
}

/// Délai entre deux événements (secondes de jeu) : de 2 h à 4 h.
pub fn next_event_delay(roll: f64) -> f64 {
    (2.0 + 2.0 * roll.clamp(0.0, 1.0)) * 3600.0
}

/// Temps laissé pour répondre à un événement.
pub const EVENT_EXPIRY: f64 = 3600.0;
/// Durée de la Convergence (temps réel, même jeu fermé).
pub const CONVERGENCE_SECS: f64 = 48.0 * 3600.0;
/// Production après la Convergence.
pub const CONVERGENCE_PRODUCTION: f64 = 10.0;

/// Texte d'un effet, pour l'interface.
pub fn effect_text(e: EventEffect) -> String {
    let h = |x: f64| {
        if x >= 1.0 {
            format!("{} h", x.round() as i64)
        } else {
            format!("{} min", (x * 60.0).round() as i64)
        }
    };
    match e {
        EventEffect::Nothing => "rien ne se passe".to_string(),
        EventEffect::Production { factor, hours } if factor >= 1.0 => {
            format!("production ×{} pendant {}", fmt_factor(factor), h(hours))
        }
        EventEffect::Production { factor, hours } => {
            format!("production ×{} pendant {}", fmt_factor(factor), h(hours))
        }
        EventEffect::Fragments { factor, hours } => {
            format!("fragments ×{} pendant {}", fmt_factor(factor), h(hours))
        }
        EventEffect::Stardust { hours } => format!("{} de production d'un coup", h(hours)),
        EventEffect::Relic { .. } => "une relique".to_string(),
        EventEffect::FleetHaste(p) => format!(
            "les flottes en route avancent de {} % d'un coup",
            (p * 100.0).round() as i64
        ),
        EventEffect::Shard => "un Éclat (+1 % de production)".to_string(),
        EventEffect::LoseStardust(p) => {
            format!("perte de {} % de ta poussière", (p * 100.0).round() as i64)
        }
        EventEffect::BuyRelic(p) => format!(
            "une relique contre {} % de ta poussière",
            (p * 100.0).round() as i64
        ),
    }
}

fn fmt_factor(f: f64) -> String {
    if f.fract() == 0.0 {
        format!("{}", f as i64)
    } else {
        format!("{f}").replace('.', ",")
    }
}

/// Les événements possibles.
pub fn catalog() -> Vec<CosmicEventDef> {
    use EventEffect as E;
    let c = |label, effect, risk| EventChoice {
        label,
        effect,
        risk,
    };
    let defs = vec![
        (
            "civ_signal",
            "Un signal venu d'ailleurs",
            "Une civilisation inconnue émet un appel à l'aide, répété en boucle depuis des millénaires.",
            vec![
                c("Lui porter secours", E::Relic { danger: 2 }, None),
                c("Lui proposer un échange", E::Stardust { hours: 3.0 }, None),
                c("Ignorer l'appel", E::Nothing, None),
            ],
        ),
        (
            "plasma_storm",
            "Tempête de plasma",
            "Un front de plasma déferle sur la fonderie. On pourrait l'abriter… ou ouvrir grand les collecteurs.",
            vec![
                c("Mettre la fonderie à l'abri", E::Nothing, None),
                c(
                    "Récolter l'orage",
                    E::Production { factor: 3.0, hours: 1.0 },
                    Some((0.7, E::Production { factor: 0.5, hours: 1.0 })),
                ),
            ],
        ),
        (
            "giant_comet",
            "Une comète géante",
            "Une comète grande comme une lune frôle ton système, sa queue chargée de poussière.",
            vec![
                c("La capturer", E::Stardust { hours: 2.0 }, None),
                c("Suivre son sillage", E::Fragments { factor: 2.0, hours: 2.0 }, None),
            ],
        ),
        (
            "wreck",
            "Une épave ancienne",
            "Un vaisseau colossal dérive, éteint depuis des âges. Ses soutes sont peut-être encore pleines.",
            vec![
                c(
                    "Fouiller les soutes",
                    E::Relic { danger: 4 },
                    Some((0.6, E::Nothing)),
                ),
                c("Le démonter pour ses matériaux", E::Stardust { hours: 1.0 }, None),
            ],
        ),
        (
            "pulsar",
            "Un pulsar s'emballe",
            "Un pulsar voisin accélère son rythme. Ses battements pourraient cadencer toute la fonderie.",
            vec![
                c(
                    "Synchroniser la fonderie",
                    E::Production { factor: 2.0, hours: 2.0 },
                    None,
                ),
                c("Le laisser battre", E::Nothing, None),
            ],
        ),
        (
            "wormhole",
            "Un trou de ver",
            "Un trou de ver s'ouvre au bord de la carte, stable pour quelques instants seulement.",
            vec![
                c("Y engager les flottes", E::FleetHaste(0.5), None),
                c("Le refermer et garder l'énergie", E::Shard, None),
            ],
        ),
        (
            "merchant",
            "Le marchand d'étoiles",
            "Un marchand itinérant propose une relique… contre la moitié de ta poussière d'étoile.",
            vec![
                c("Accepter le marché", E::BuyRelic(0.5), None),
                c("Refuser poliment", E::Nothing, None),
            ],
        ),
        (
            "nearby_nova",
            "Une supernova voisine",
            "Une étoile proche explose. L'onde arrive : on peut s'en protéger, ou s'y baigner.",
            vec![
                c("Se protéger", E::Nothing, None),
                c(
                    "Absorber l'onde",
                    E::Fragments { factor: 3.0, hours: 1.0 },
                    Some((0.75, E::LoseStardust(0.25))),
                ),
            ],
        ),
    ];
    defs.into_iter()
        .enumerate()
        .map(|(index, (key, title, text, choices))| CosmicEventDef {
            index,
            key,
            title,
            text,
            choices,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_and_texts() {
        let c = catalog();
        assert_eq!(c.len(), 8);
        assert!(c.iter().all(|e| (2..=3).contains(&e.choices.len())));
        assert_eq!(
            effect_text(EventEffect::Production {
                factor: 3.0,
                hours: 1.0
            }),
            "production ×3 pendant 1 h"
        );
        assert_eq!(
            effect_text(EventEffect::Production {
                factor: 0.5,
                hours: 0.5
            }),
            "production ×0,5 pendant 30 min"
        );
        let d = next_event_delay(0.5);
        assert!((d - 3.0 * 3600.0).abs() < 1e-9);
    }
}
