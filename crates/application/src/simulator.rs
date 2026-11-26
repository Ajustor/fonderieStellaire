//! Simulateur d'équilibrage : rejoue une partie avec le **vrai** code du domaine
//! et une stratégie automatique raisonnable, et note quand chaque étape est
//! atteinte (en temps de jeu continu). Sert à vérifier le rythme d'une version
//! avant de la publier : `fonderie-stellaire --simulate`.
//!
//! Stratégie : clics réguliers, comètes toutes capturées, améliorations dès
//! qu'abordables, générateurs au meilleur rapport production / coût, Atelier
//! du moins cher au plus cher puis Constellations, Supernova dès qu'elle double
//! les fragments (5 minimum), Big Crunch dès que possible (après la Clé de
//! l'univers en cours), recherches du Laboratoire dans l'ordre le moins cher,
//! Big Bang à 30 Big Crunch, forges et Grand Attracteur dès que possible.
//! Les Défis (facultatifs) ne sont pas joués.

use domain::{economy, BuyMode, Catalog, DomainEvent, GameState, GeneratorId, UniverseId};

#[derive(Clone, Copy, Debug)]
pub struct SimConfig {
    /// Durée maximale simulée (secondes de jeu).
    pub max_secs: f64,
    /// Clics manuels par seconde.
    pub clicks_per_sec: f64,
    /// Graine du hasard (comètes).
    pub seed: u64,
    /// Heures jouées par jour ; le reste du temps, le jeu est fermé (gains
    /// hors-ligne). `None` : jeu continu. Les durées sont alors en jours réels.
    pub daily_hours: Option<f64>,
}

impl Default for SimConfig {
    fn default() -> Self {
        SimConfig {
            max_secs: 60.0 * 24.0 * 3600.0,
            clicks_per_sec: 1.0,
            seed: 0x5EED_F00D,
            daily_hours: None,
        }
    }
}

/// Une étape atteinte et l'instant (secondes de jeu).
#[derive(Clone, Debug, PartialEq)]
pub struct Milestone {
    pub label: String,
    pub at: f64,
}

#[derive(Clone, Debug, Default)]
pub struct SimReport {
    pub milestones: Vec<Milestone>,
    /// Temps simulé au total.
    pub simulated: f64,
    /// La partie est allée jusqu'à la fin du contenu (carte complète).
    pub finished: bool,
    pub supernovas_total: u32,
    pub crunches_total: u32,
    /// Succès débloqués : (clé, instant), dans l'ordre d'obtention.
    pub achievements: Vec<(&'static str, f64)>,
}

/// Hasard déterministe (xorshift), indépendant de l'infrastructure.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Pas de temps : fin au début (achats fréquents), plus large ensuite.
fn step_for(t: f64) -> f64 {
    if t < 2.0 * 3600.0 {
        1.0
    } else if t < 48.0 * 3600.0 {
        3.0
    } else {
        5.0
    }
}

pub fn simulate(catalog: &Catalog, cfg: &SimConfig, mut progress: impl FnMut(f64)) -> SimReport {
    let mut s = GameState::new(catalog);
    let mut rng = Rng(cfg.seed.max(1));
    let mut ev: Vec<DomainEvent> = Vec::new();
    let mut report = SimReport::default();
    let mut seen: Vec<String> = Vec::new();
    let mut mark = |report: &mut SimReport, label: String, at: f64| {
        if !seen.contains(&label) {
            seen.push(label.clone());
            report.milestones.push(Milestone { label, at });
        }
    };
    let mut click_debt = 0.0;
    let mut last_progress = 0.0;
    let mut last_nova = 0.0;
    let mut upgrade_timer = 0.0;
    let mut t = 0.0;

    let mut session = 0.0;
    while t < cfg.max_secs {
        let dt = step_for(t);
        t += dt;
        ev.clear();

        // Joueur réel : après sa séance du jour, le jeu reste fermé jusqu'au lendemain.
        if let Some(h) = cfg.daily_hours {
            session += dt;
            if session >= h * 3600.0 {
                session = 0.0;
                let away = (24.0 - h).max(0.0) * 3600.0;
                s.apply_offline(catalog, away, &mut ev);
                t += away;
            }
        }

        // Clics et comètes.
        click_debt += cfg.clicks_per_sec * dt;
        while click_debt >= 1.0 {
            click_debt -= 1.0;
            s.harvest(catalog, &mut ev);
        }
        s.tick(catalog, dt, rng.next(), &mut ev);
        if s.comet.active.is_some() {
            s.catch_comet(catalog, rng.next(), rng.next(), &mut ev);
        }

        // Achats.
        upgrade_timer += dt;
        if upgrade_timer >= 5.0 {
            upgrade_timer = 0.0;
            s.buy_all_upgrades(catalog, &mut ev);
        }
        buy_generators(catalog, &mut s, &mut ev);
        buy_prestige_shop(catalog, &mut s, &mut ev);

        // Laboratoire : la recherche la moins chère disponible.
        if s.research.is_none() {
            if let Some(id) = catalog
                .lab
                .iter()
                .filter(|l| s.can_research(catalog, l.id))
                .min_by_key(|l| (l.cost, l.research_secs as u64))
                .map(|l| l.id)
            {
                s.start_research(catalog, id, &mut ev);
            }
        }

        // Supernova dès qu'elle double les fragments.
        let gain = s.fragments_available_with(catalog);
        if s.challenge.is_none() && gain >= 5.0f64.max(s.fragments) {
            s.supernova(catalog, &mut ev);
            last_nova = t;
        }

        // Big Crunch : après la Clé de l'univers en cours (ou si on stagne).
        if s.can_big_crunch() {
            let pending_key = catalog.universe(s.universe).is_some_and(|u| {
                u.key_reward.is_some() && !s.has_universe_key(u.id) && s.supernovas < u.key_goal
            });
            let stuck = t - last_nova > 6.0 * 3600.0;
            if !pending_key || stuck {
                let next = catalog
                    .universes
                    .iter()
                    .find(|u| u.key_reward.is_some() && !s.has_universe_key(u.id))
                    .map(|u| u.id)
                    .unwrap_or(UniverseId::ORIGIN);
                s.big_crunch(catalog, next, &mut ev);
                last_nova = t;
            }
        }

        // Big Bang, forges, Grand Attracteur.
        if s.can_big_bang() {
            s.big_bang(catalog, &mut ev);
        }
        for g in catalog.generators.iter().filter(|g| g.is_primordial()) {
            if s.can_forge(catalog, g.id) {
                s.forge(catalog, g.id, &mut ev);
            }
        }
        if s.can_build_attractor(catalog) {
            s.build_attractor(catalog, &mut ev);
        }

        // Expéditions : d'abord explorer (la cible la plus sûre), sinon
        // prospecter le système exploré le plus dangereux (meilleures reliques).
        while s.fleets.len() < s.fleets_max() {
            let unexplored = (1..s.map.systems.len())
                .filter(|&i| !s.is_explored(i) && s.can_launch_expedition(catalog, i))
                .max_by(|&a, &b| {
                    s.expedition_chance_in(catalog, a)
                        .partial_cmp(&s.expedition_chance_in(catalog, b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            let target = unexplored.or_else(|| {
                (1..s.map.systems.len())
                    .filter(|&i| s.is_explored(i) && s.can_launch_expedition(catalog, i))
                    .max_by_key(|&i| s.map.systems[i].danger)
            });
            match target {
                Some(i) => s.launch_expedition(catalog, i, rng.next(), &mut ev),
                None => break,
            }
        }

        // Événements : premier choix sans risque ; Convergence dès que possible.
        if s.pending_event.is_some() {
            let choice = s
                .pending_event
                .and_then(|(i, _)| catalog.events.get(i))
                .and_then(|e| e.choices.iter().position(|c| c.risk.is_none()))
                .unwrap_or(0);
            s.resolve_event(catalog, choice, rng.next(), &mut ev);
        }
        if s.convergence_ready(catalog) {
            s.start_convergence(catalog, &mut ev);
        }

        // Mégastructures : chaque heure, la meilleure construction abordable.
        if s.megastructures_unlocked() && (t / 3600.0).fract() < dt / 3600.0 + 1e-9 {
            build_megastructures(catalog, &mut s, &mut ev);
        }
        if s.awakening_ready() {
            s.start_awakening(&mut ev);
        }

        // Reliques : on équipe les plus rares, en privilégiant la production.
        if ev
            .iter()
            .any(|e| matches!(e, DomainEvent::RelicFound { .. }))
        {
            let mut owned: Vec<_> = catalog
                .relics
                .iter()
                .filter_map(|r| s.relic_rarity(r.id).map(|k| (r.id, k, r.set)))
                .collect();
            owned.sort_by_key(|&(_, k, set)| {
                (
                    std::cmp::Reverse(k),
                    matches!(set, domain::RelicSet::Eclipse) as u8,
                )
            });
            for id in s.equipped.clone() {
                s.toggle_relic(catalog, id, &mut ev);
            }
            for (id, _, _) in owned.into_iter().take(s.relic_slots()) {
                s.toggle_relic(catalog, id, &mut ev);
            }
        }

        // Étapes.
        for e in &ev {
            match e {
                DomainEvent::Supernova { .. } => {
                    report.supernovas_total += 1;
                    let n = report.supernovas_total;
                    if n == 1 {
                        mark(&mut report, "1re Supernova".into(), t);
                    } else if n == 15 {
                        mark(&mut report, "15e Supernova".into(), t);
                    }
                }
                DomainEvent::BigCrunch { crunches } => {
                    report.crunches_total += 1;
                    if s.big_bangs == 0 && matches!(crunches, 1 | 10 | 30) {
                        let label = if *crunches == 1 {
                            "1er Big Crunch".to_string()
                        } else {
                            format!("{crunches}e Big Crunch")
                        };
                        mark(&mut report, label, t);
                    }
                }
                DomainEvent::LabResearchDone { .. } if s.lab.iter().all(|&d| d) => {
                    mark(&mut report, "Laboratoire complet".into(), t);
                }
                DomainEvent::UniverseKeyEarned { universe } => {
                    let name = catalog
                        .universe(*universe)
                        .map(|u| u.key_name)
                        .unwrap_or("?");
                    mark(&mut report, name.to_string(), t);
                }
                DomainEvent::BigBang { .. } => {
                    let n = s.big_bangs;
                    if n == 1 || n.is_multiple_of(5) {
                        mark(&mut report, format!("Big Bang n°{n}"), t);
                    }
                }
                DomainEvent::GeneratorForged { generator } => {
                    let name = catalog.generator(*generator).map(|g| g.name).unwrap_or("?");
                    mark(&mut report, format!("Forge : {name}"), t);
                }
                DomainEvent::AttractorStageDone { stage } => {
                    let name = catalog
                        .attractor_stage(*stage)
                        .map(|a| a.name)
                        .unwrap_or("?");
                    mark(
                        &mut report,
                        format!("Attracteur {}/7 : {name}", stage + 1),
                        t,
                    );
                }
                DomainEvent::AttractorAwakened => {
                    mark(&mut report, "Fin de l'Acte II".into(), t);
                }
                DomainEvent::ExpeditionReturned { success: true, .. } => {
                    let n = s.explored_count();
                    if n >= 1 {
                        mark(&mut report, "Carte : 1er système exploré".into(), t);
                    }
                    if n >= 10 {
                        mark(&mut report, "Carte : 10 systèmes explorés".into(), t);
                    }
                }
                DomainEvent::AchievementUnlocked { achievement } => {
                    if let Some(a) = catalog.achievement(*achievement) {
                        report.achievements.push((a.key, t));
                    }
                }
                DomainEvent::StarMapCompleted => {
                    mark(&mut report, "Carte stellaire complète".into(), t);
                }
                DomainEvent::RelicFound { .. } => {
                    let n = s.relics_owned();
                    if n >= 1 {
                        mark(&mut report, "1re relique".into(), t);
                    }
                    if n >= 8 {
                        mark(&mut report, "8 reliques".into(), t);
                    }
                    if n >= catalog.relics.len() {
                        mark(&mut report, "Collection de reliques complète".into(), t);
                    }
                }
                DomainEvent::CosmicEventResolved { .. } => {
                    if s.events_resolved == 1 {
                        mark(&mut report, "1er événement cosmique".into(), t);
                    }
                }
                DomainEvent::ConvergenceStarted => {
                    mark(&mut report, "Convergence lancée".into(), t);
                }
                DomainEvent::ConvergenceDone => {
                    mark(&mut report, "Fin de l'Acte III (Convergence)".into(), t);
                }
                DomainEvent::StructurePlaced { kind, .. } => {
                    let n = s.structures_built();
                    if n == 1 {
                        mark(&mut report, "1re mégastructure".into(), t);
                    }
                    if *kind == domain::StructureKind::Core {
                        mark(&mut report, "Cœur stellaire".into(), t);
                    }
                    if n >= 20 {
                        mark(&mut report, "20 mégastructures".into(), t);
                    }
                    if n >= domain::megastructure::CELLS {
                        mark(&mut report, "Grille complète".into(), t);
                    }
                }
                DomainEvent::AwakeningStarted => {
                    mark(&mut report, "Éveil du Cosmos lancé".into(), t);
                }
                DomainEvent::CosmosAwakened => {
                    mark(
                        &mut report,
                        "Fin de l'Acte IV (le Cosmos s'éveille)".into(),
                        t,
                    );
                    report.finished = true;
                }
                _ => {}
            }
        }
        if report.finished {
            break;
        }
        if t - last_progress >= 3600.0 {
            last_progress = t;
            progress(t);
        }
    }
    report.simulated = t;
    report
}

/// Note d'une grille, pour le bâtisseur du simulateur.
fn grid_score(g: &[Option<domain::StructureKind>]) -> f64 {
    let e = domain::megastructure::effects(g);
    e.exotic_per_hour * 2.0
        + e.production.ln() * 30.0
        + e.fragments.ln() * 10.0
        + e.expedition_speed * 10.0
}

/// Bâtit, tant que c'est abordable, la structure qui améliore le plus la grille
/// (par matière exotique dépensée).
fn build_megastructures(catalog: &Catalog, s: &mut GameState, ev: &mut Vec<DomainEvent>) {
    for _ in 0..10 {
        let base = grid_score(&s.grid);
        let mut best: Option<(f64, usize, domain::StructureKind)> = None;
        for kind in domain::StructureKind::ALL {
            let cost = s.structure_cost(kind);
            if s.exotic < cost || (kind.unique() && s.structure_count(kind) > 0) {
                continue;
            }
            for cell in 0..s.grid.len() {
                if s.grid[cell].is_some() {
                    continue;
                }
                let mut g = s.grid.clone();
                g[cell] = Some(kind);
                let gain = (grid_score(&g) - base) / cost;
                if best.is_none_or(|(b, _, _)| gain > b) {
                    best = Some((gain, cell, kind));
                }
            }
        }
        match best {
            Some((gain, cell, kind)) if gain > 0.0 => s.place_structure(catalog, cell, kind, ev),
            _ => break,
        }
    }
}

/// Générateurs au meilleur rapport production / coût, tant que c'est abordable.
fn buy_generators(catalog: &Catalog, s: &mut GameState, ev: &mut Vec<DomainEvent>) {
    let visible: Vec<GeneratorId> = catalog
        .generators
        .iter()
        .filter(|g| s.is_visible(catalog, g.id))
        .map(|g| g.id)
        .collect();
    let mults: Vec<f64> = visible
        .iter()
        .map(|&id| economy::generator_multiplier(catalog, s, id))
        .collect();
    for _ in 0..25 {
        let best = visible
            .iter()
            .zip(&mults)
            .filter_map(|(&id, &m)| {
                let def = catalog.generator(id)?;
                let cost = s.generator_cost(catalog, id, 1);
                Some((id, def.base_prod * m / cost, cost))
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        match best {
            Some((id, _, cost)) if s.stardust.can_afford(cost) => {
                s.buy_generator(catalog, id, BuyMode::One, ev);
            }
            _ => break,
        }
    }
}

/// Atelier du moins cher au plus cher, puis Constellations.
fn buy_prestige_shop(catalog: &Catalog, s: &mut GameState, ev: &mut Vec<DomainEvent>) {
    let mut perks: Vec<_> = catalog.perks.iter().filter(|p| !s.has_perk(p.id)).collect();
    perks.sort_by(|a, b| {
        a.cost
            .partial_cmp(&b.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if let Some(p) = perks.first() {
        if s.fragments_spendable() + 1e-9 >= p.cost {
            s.buy_perk(catalog, p.id, ev);
        }
        return;
    }
    for _ in 0..10 {
        let next = catalog
            .constellations
            .iter()
            .filter(|c| !c.is_maxed(s.constellation_level(c.id)))
            .map(|c| (c.id, c.cost_at(s.constellation_level(c.id))))
            .filter(|&(_, cost)| s.fragments_spendable() + 1e-9 >= cost)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        match next {
            Some((id, _)) => s.buy_constellation(catalog, id, ev),
            None => break,
        }
    }
}

/// Durée lisible : `3 j 04 h`, `5 h 12 min`, `42 min`.
pub fn fmt_play_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    let (d, h, m) = (s / 86_400, s % 86_400 / 3600, s % 3600 / 60);
    if d > 0 {
        format!("{d} j {h:02} h")
    } else if h > 0 {
        format!("{h} h {m:02} min")
    } else {
        format!("{m} min")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_game_reaches_the_first_supernova() {
        let c = Catalog::standard();
        let cfg = SimConfig {
            max_secs: 6.0 * 3600.0,
            ..SimConfig::default()
        };
        let r = simulate(&c, &cfg, |_| {});
        let first = r
            .milestones
            .iter()
            .find(|m| m.label == "1re Supernova")
            .expect("première Supernova en moins de 6 h");
        assert!(first.at > 5.0 * 60.0, "pas trop vite : {}", first.at);
    }

    #[test]
    fn play_time_is_readable() {
        assert_eq!(fmt_play_time(42.0 * 60.0), "42 min");
        assert_eq!(fmt_play_time(5.0 * 3600.0 + 12.0 * 60.0), "5 h 12 min");
        assert_eq!(fmt_play_time(3.0 * 86400.0 + 4.0 * 3600.0), "3 j 04 h");
    }

    /// Ordre d'obtention des succès sur une partie simulée (3 h par jour).
    /// `cargo test --release -p application achievement_order -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn achievement_order() {
        let c = Catalog::standard();
        let cfg = SimConfig {
            daily_hours: Some(3.0),
            max_secs: 60.0 * 86_400.0,
            ..SimConfig::default()
        };
        let r = simulate(&c, &cfg, |_| {});
        for (k, t) in &r.achievements {
            println!("{k:<14} {}", fmt_play_time(*t));
        }
        let missing: Vec<_> = c
            .achievements
            .iter()
            .filter(|a| !r.achievements.iter().any(|(k, _)| *k == a.key))
            .map(|a| a.key)
            .collect();
        println!("jamais : {missing:?}");
    }
}
