//! Écrans du mode mobile, en portrait (40 à 60 colonnes). Chaque bouton
//! enregistre sa zone dans la `HitMap` au moment où il est dessiné.

use application::GameSnapshot;
use domain::CometKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use super::{Confirm, Detail, Hit, HitMap, MobileUi, Page, Press, Request, Tab};
use crate::bigfont;
use crate::draw;
use crate::fmt;
use crate::fx::easing::ease_out_cubic;
use crate::fx::{effects, Rgb};
use crate::input::Intent;
use crate::theme;
use crate::ui::Ui;
use crate::visuals::Visuals;
use crate::widgets::journal::{self, Journal};
use crate::widgets::shop::{self, ShopItem};
use crate::widgets::toast::Toasts;
use crate::widgets::{codex, generators, header, sky, sparkline, upgrades, Ctx};

/// Écrans défilants (index dans `MobileUi::scroll`).
pub const FACTORIES: usize = 0;
pub const UPGRADES: usize = 1;
pub const ATELIER: usize = 2;
pub const CONSTELLATIONS: usize = 3;
pub const COSMOS: usize = 4;
pub const JOURNAL: usize = 5;
pub const INFOS: usize = 6;
pub const OPTIONS: usize = 7;
pub const SAVE: usize = 8;
pub const FOUNDRY: usize = 9;

/// Hauteur de l'en-tête commun (titre, compteur, ressources, progression).
pub const HEADER_H: u16 = 6;
/// Hauteur d'un bouton (bordure, libellé, bordure) : ≈ 48 dp en portrait.
pub(super) const BUTTON_H: u16 = 3;
/// Une carte : 3 lignes de texte + 1 ligne de séparation.
const CARD_H: u16 = 4;
/// Largeur du bouton d'achat à droite des cartes.
const BUY_W: u16 = 13;
/// Panneau du journal sur l'onglet Fonderie (deux lignes visibles).
const JOURNAL_H: u16 = 4;
/// Fondu d'entrée d'un onglet.
const TAB_FADE: f64 = 0.15;

/// Ce dont les écrans ont besoin en plus du contexte de rendu.
pub struct MobileView<'a> {
    pub journal: &'a Journal,
    pub toasts: &'a Toasts,
    pub spark_scratch: &'a mut Vec<u64>,
    pub version: &'a str,
    /// Jour (depuis 1970), pour le Défi du jour.
    pub today: u64,
}

/// Sous-onglets de l'onglet `tab`, selon les déblocages.
pub fn sub_tabs(snap: &GameSnapshot, tab: Option<Tab>) -> Vec<&'static str> {
    match tab {
        Some(Tab::Upgrade) => {
            let mut v = vec!["Améliorations"];
            if atelier_unlocked(snap) {
                v.push("Atelier");
                if !snap.constellations.is_empty() {
                    v.push("Constellations");
                }
            }
            v
        }
        Some(Tab::More) => vec!["Journal", "Infos", "Options", "Sauvegarde"],
        _ => Vec::new(),
    }
}

fn atelier_unlocked(snap: &GameSnapshot) -> bool {
    snap.supernovas > 0 || snap.fragments > 0.0 || snap.crunches > 0
}

/// Les textes du jeu citent des touches : version tactile.
pub fn touchify(s: &str) -> String {
    s.replace("avec [1] à [6]", "dans la palette en bas")
        .replace("[D] démolir", "Démolir")
        .replace("Maintenir [Espace]", "Maintenir le doigt")
        .replace("maintenir [Espace]", "maintenir le doigt")
        .replace("[Espace]", "toucher le ciel")
}

pub fn render(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    vis: &mut Visuals,
    ui: &mut Ui,
    m: &mut MobileUi,
    view: MobileView,
) {
    m.hits.clear();
    let header_rect = Rect::new(area.x, area.y, area.width, HEADER_H.min(area.height));
    render_header(buf, header_rect, ctx, vis);
    ui.layout.header = header_rect;

    let mut content = Rect::new(
        area.x,
        header_rect.bottom(),
        area.width,
        area.height.saturating_sub(header_rect.height),
    );
    // Sous-onglets : une rangée de boutons en haut de l'écran.
    let subs = sub_tabs(ctx.snap, Some(m.tab));
    let t = m.tab.index();
    if !subs.is_empty() {
        m.sub[t] = m.sub[t].min(subs.len() - 1);
        let row = Rect::new(content.x, content.y, content.width, BUTTON_H);
        render_sub_tabs(buf, row, &subs, m.sub[t], &mut m.hits, m.press);
        content.y += BUTTON_H;
        content.height = content.height.saturating_sub(BUTTON_H);
    }
    // Les effets (étincelles d'achat, toasts) partent des zones de l'écran affiché.
    ui.layout.generators = content;
    ui.layout.upgrades = content;
    ui.layout.sky = content;
    ui.gen_rows = [None; theme::GEN_SLOTS];
    ui.upg_rows.clear();

    if m.tab != Tab::Cosmos {
        m.page = None;
    }
    let screen = match (m.tab, m.sub[t]) {
        (Tab::Cosmos, _) if m.page.is_some() => {
            super::cosmos::page_screen(m.page.unwrap_or(Page::Event))
        }
        (Tab::Foundry, _) => FOUNDRY,
        (Tab::Factories, _) => FACTORIES,
        (Tab::Upgrade, 0) => UPGRADES,
        (Tab::Upgrade, 1) => ATELIER,
        (Tab::Upgrade, _) => CONSTELLATIONS,
        (Tab::Cosmos, _) => COSMOS,
        (Tab::More, 0) => JOURNAL,
        (Tab::More, 1) => INFOS,
        (Tab::More, 2) => OPTIONS,
        (Tab::More, _) => SAVE,
    };
    m.screen = screen;
    match screen {
        FOUNDRY => render_foundry(buf, content, ctx, vis, ui, m, &view),
        FACTORIES => render_factories(buf, content, ctx, vis, ui, m, view.spark_scratch),
        UPGRADES => render_upgrades(buf, content, ctx, ui, m),
        ATELIER => render_atelier(buf, content, ctx, m),
        CONSTELLATIONS => render_constellations(buf, content, ctx, m),
        COSMOS => render_cosmos(buf, content, ctx, m),
        _ if m.page.is_some() => {
            let page = m.page.unwrap_or(Page::Event);
            super::cosmos::render_page(buf, content, ctx, vis, m, page, view.today);
        }
        JOURNAL => {
            journal::render(buf, content, view.journal, ctx.now, &mut ui.journal_scroll);
            retitle(buf, content, " Journal ", "glisser pour remonter ");
        }
        INFOS => render_infos(buf, content, ctx, m),
        OPTIONS => render_options(buf, content, ctx, m),
        _ => render_save(buf, content, m, view.version),
    }

    let fade = (ctx.now - m.tab_changed_at) / TAB_FADE;
    if (0.0..1.0).contains(&fade) {
        effects::fade(buf, content, 1.0 - ease_out_cubic(fade));
    }
    vis.particles.draw(buf, area);
    view.toasts.render(buf, area, content.y, ctx.now);
    if let Some(d) = m.detail {
        render_detail(buf, area, ctx, d, m);
    }
    if let Some(c) = m.confirm {
        super::cosmos::render_confirm(buf, area, ctx, m, c);
    }
}

// ----------------------------------------------------------------------
// Briques communes
// ----------------------------------------------------------------------

/// Bouton bordé de `r.height` lignes, libellé centré, titre optionnel sur la bordure.
#[allow(clippy::too_many_arguments)]
pub(super) fn button(
    buf: &mut Buffer,
    hits: &mut HitMap,
    press: Option<Press>,
    r: Rect,
    title: &str,
    label: &str,
    color: Rgb,
    hit: Option<Hit>,
) {
    if r.width < 3 || r.height < 3 {
        return;
    }
    let pressed = press.is_some_and(|p| p.rect == r);
    draw::clear(buf, r);
    if pressed {
        draw::fill_bg(buf, r, color.lerp(theme::BLACK, 0.6));
    }
    draw::card(buf, r, r, color);
    let inner_w = r.width.saturating_sub(2) as usize;
    if !title.is_empty() {
        let t = draw::truncate(&format!(" {title} "), inner_w);
        let x = r.x + (r.width.saturating_sub(t.chars().count() as u16)) / 2;
        draw::text(buf, x, r.y, &t, theme::fg(color), r.right() - 1);
    }
    let l = draw::truncate(label, inner_w);
    let x = r.x + (r.width.saturating_sub(l.chars().count() as u16)) / 2;
    let y = r.y + r.height / 2;
    let style = if pressed {
        theme::bold(theme::WHITE)
    } else {
        theme::bold(color)
    };
    draw::text(buf, x, y, &l, style, r.right() - 1);
    if let Some(hit) = hit {
        hits.add(r, hit);
    }
}

fn render_sub_tabs(
    buf: &mut Buffer,
    row: Rect,
    subs: &[&str],
    selected: usize,
    hits: &mut HitMap,
    press: Option<Press>,
) {
    // Largeur selon le libellé, l'espace restant réparti également.
    let n = subs.len() as u16;
    let natural: Vec<u16> = subs.iter().map(|s| s.chars().count() as u16 + 4).collect();
    let sum: u16 = natural.iter().sum();
    let widths: Vec<u16> = if sum <= row.width {
        let extra = row.width - sum;
        natural
            .iter()
            .enumerate()
            .map(|(i, w)| w + extra / n + u16::from((i as u16) < extra % n))
            .collect()
    } else {
        (0..n)
            .map(|i| row.width / n + u16::from(i < row.width % n))
            .collect()
    };
    let mut x = row.x;
    for (i, name) in subs.iter().enumerate() {
        let r = Rect::new(x, row.y, widths[i], row.height);
        x += widths[i];
        let color = if i == selected {
            theme::FOCUS
        } else {
            theme::BORDER
        };
        button(buf, hits, press, r, "", name, color, Some(Hit::SubTab(i)));
    }
}

/// Place une carte de hauteur `h` à l'ordonnée virtuelle `vy` d'une liste
/// défilante ; `None` si elle n'est pas entièrement visible.
pub(super) fn place(area: Rect, scroll: u16, vy: u16, h: u16) -> Option<Rect> {
    let y = vy.checked_sub(scroll)?;
    if y + h > area.height {
        return None;
    }
    // Dernière colonne réservée à l'indicateur de défilement.
    Some(Rect::new(
        area.x,
        area.y + y,
        area.width.saturating_sub(1),
        h,
    ))
}

pub(super) fn set_scroll_max(m: &mut MobileUi, screen: usize, total: u16, visible: u16) {
    m.scroll_max[screen] = total.saturating_sub(visible);
    m.scroll[screen] = m.scroll[screen].min(m.scroll_max[screen]);
}

/// Indicateur de défilement sur le bord droit.
pub(super) fn scrollbar(buf: &mut Buffer, area: Rect, scroll: u16, max: u16) {
    if max == 0 || area.height < 3 {
        return;
    }
    let h = area.height;
    let pos = (scroll as u32 * (h - 1) as u32 / max as u32) as u16;
    let x = area.right().saturating_sub(1);
    draw::put(
        buf,
        area,
        x as i32,
        (area.y + pos) as i32,
        '▐',
        theme::MUTED,
    );
}

/// Remplace le titre d'un panneau du terminal (qui cite des touches).
fn retitle(buf: &mut Buffer, r: Rect, title: &str, right: &str) {
    if r.width < 4 {
        return;
    }
    for x in r.x + 1..r.right() - 1 {
        draw::put(buf, r, x as i32, r.y as i32, '─', theme::BORDER);
    }
    draw::text(
        buf,
        r.x + 1,
        r.y,
        title,
        theme::title_style(false),
        r.right() - 1,
    );
    draw::text_right(
        buf,
        r.right() - 2,
        r.y,
        right,
        theme::fg(theme::MUTED),
        r.x + 2,
    );
}

pub(super) fn separator(buf: &mut Buffer, x: u16, y: u16, w: u16) {
    for i in 0..w {
        draw::put(
            buf,
            buf.area,
            (x + i) as i32,
            y as i32,
            '┄',
            theme::BAR_EMPTY,
        );
    }
}

// ----------------------------------------------------------------------
// En-tête
// ----------------------------------------------------------------------

fn render_header(buf: &mut Buffer, area: Rect, ctx: &Ctx, vis: &mut Visuals) {
    if area.height < HEADER_H {
        return;
    }
    let snap = ctx.snap;
    let now = ctx.now;
    let right = area.right().saturating_sub(1);
    header::gradient_title(buf, area.x + 1, area.y, "✦ FONDERIE STELLAIRE", now, right);
    let pct = fmt::percent(snap.progress.completion);
    draw::text_right(buf, right, area.y, &pct, theme::fg(theme::GOLD), area.x);

    // Colonne de droite : production, clic, mot d'échelle.
    let prod = format!("+{} /s", fmt::short(snap.production));
    let mut limits = [right; 3];
    limits[0] = if snap.storm_remaining > 0.0 {
        let golden = snap.storm_multiplier >= 50.0;
        let base = if golden { theme::GOLD } else { theme::ORANGE };
        let beat = (now * std::f64::consts::PI * 2.0).sin().abs();
        let storm = format!(
            "×{} {}",
            fmt::short(snap.storm_multiplier),
            fmt::countdown(snap.storm_remaining)
        );
        let x = draw::text_right(
            buf,
            right,
            area.y + 1,
            &storm,
            theme::bold(base.lerp(theme::WHITE, 0.35 * beat)),
            area.x,
        );
        draw::text_right(
            buf,
            x.saturating_sub(1),
            area.y + 2,
            &prod,
            theme::bold(theme::GREEN),
            area.x,
        );
        x
    } else {
        draw::text_right(
            buf,
            right,
            area.y + 1,
            &prod,
            theme::bold(theme::GREEN),
            area.x,
        )
    };
    let click = format!("clic +{}", fmt::short(snap.click_value));
    if snap.storm_remaining <= 0.0 {
        limits[1] = draw::text_right(
            buf,
            right,
            area.y + 2,
            &click,
            theme::fg(theme::MUTED),
            area.x,
        );
    } else {
        limits[1] = right.saturating_sub(16);
    }

    // Gros compteur.
    let shown = vis.counter_value(now, snap.stardust);
    let (digits, word) = fmt::big(shown);
    vis.track_digits(&digits, now);
    let x0 = area.x + 1;
    let total = digits.chars().count();
    let bump = ((now - vis.bump_at) / 0.15).clamp(0.0, 1.0);
    let bump_glow = if bump < 1.0 {
        0.65 * (1.0 - ease_out_cubic(bump))
    } else {
        0.0
    };
    let max_x = limits
        .iter()
        .copied()
        .min()
        .unwrap_or(right)
        .saturating_sub(1);
    let changed = vis.digit_changed;
    bigfont::layout(&digits, |dx, dy, ci, ch| {
        let x = x0 + dx;
        if x >= max_x {
            return;
        }
        let mut c = theme::logo_color(dx as f64, now);
        if let Some(&t) = changed.get(total - 1 - ci) {
            let p = (now - t) / 0.25;
            if (0.0..1.0).contains(&p) {
                c = c.glow(0.45 * (1.0 - p));
            }
        }
        draw::put(
            buf,
            area,
            x as i32,
            (area.y + 1 + dy) as i32,
            ch,
            c.glow(bump_glow),
        );
    });
    let word_x = x0 + bigfont::text_width(&digits) + 1;
    if !word.starts_with("poussi") {
        draw::text(
            buf,
            word_x,
            area.y + 3,
            &word,
            theme::bold(theme::GOLD),
            right.saturating_sub(1),
        );
    }

    // Ressources de prestige, ou le Défi en cours.
    let line = match snap.active_challenge {
        Some(a) => {
            let name = ctx.catalog.challenge(a.id).map(|c| c.name).unwrap_or("?");
            let time = a
                .time_left
                .map(|t| format!(" · {}", fmt::countdown(t)))
                .unwrap_or_default();
            (
                format!("◆ DÉFI {name} {}{time}", fmt::percent(a.progress)),
                theme::PINK,
            )
        }
        None => {
            let mut parts = Vec::new();
            if snap.fragments > 0.0 {
                parts.push(format!(
                    "◆ {} (+{})",
                    fmt::short(snap.fragments),
                    fmt::percent(snap.fragment_bonus)
                ));
            }
            if snap.crunches > 0 {
                parts.push(format!("✶ {}", snap.crunches));
            }
            if snap.big_bangs > 0 {
                parts.push(format!("✷ {}", snap.particles));
            }
            parts.push(format!(
                "★ {}/{}",
                snap.achievements_unlocked, snap.achievements_total
            ));
            if let Some(d) = snap.daily {
                parts.insert(0, format!("✧ {}", fmt::countdown(d.left)));
            }
            (parts.join("  "), theme::CYAN)
        }
    };
    let text = draw::truncate(&line.0, area.width.saturating_sub(2) as usize);
    draw::text(buf, area.x + 1, area.y + 4, &text, theme::fg(line.1), right);

    // Barre de progression globale.
    let w = area.width.saturating_sub(2);
    let filled = (snap.progress.completion.clamp(0.0, 1.0) * w as f64).round() as u16;
    for i in 0..w {
        let (ch, c) = if i < filled {
            (
                '━',
                theme::GOLD.lerp(theme::PINK, i as f64 / w.max(1) as f64),
            )
        } else {
            ('─', theme::BAR_EMPTY)
        };
        draw::put(
            buf,
            area,
            (area.x + 1 + i) as i32,
            (area.y + 5) as i32,
            ch,
            c,
        );
    }
}

// ----------------------------------------------------------------------
// Fonderie
// ----------------------------------------------------------------------

fn render_foundry(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    vis: &mut Visuals,
    ui: &mut Ui,
    m: &mut MobileUi,
    view: &MobileView,
) {
    let snap = ctx.snap;
    let bottom_h = JOURNAL_H + BUTTON_H;
    let sky_rect = Rect::new(
        area.x,
        area.y,
        area.width,
        area.height.saturating_sub(bottom_h),
    );
    ui.layout.sky = sky_rect;
    sky::render(buf, sky_rect, ctx, vis);
    m.hits.add(crate::widgets::inner(sky_rect), Hit::Harvest);

    // Comète : un gros bouton dans le bas du ciel.
    if snap.comet_progress.is_some() {
        let (label, color) = match snap.comet_kind {
            CometKind::Golden => ("● Comète dorée : vite !", theme::GOLD),
            CometKind::Black => ("● Comète noire : attrape-la !", theme::PINK),
            _ => ("● Capturer la comète !", theme::CYAN),
        };
        let pulse = 0.5 + 0.5 * (ctx.now * 6.0).sin();
        let w = 32.min(sky_rect.width.saturating_sub(4));
        let r = Rect::new(
            sky_rect.x + (sky_rect.width - w) / 2,
            sky_rect.bottom().saturating_sub(BUTTON_H + 1),
            w,
            BUTTON_H,
        );
        button(
            buf,
            &mut m.hits,
            m.press,
            r,
            "",
            label,
            color.glow(0.3 * pulse),
            Some(Hit::CatchComet),
        );
    }

    // Dernières lignes du journal ; le toucher ouvre l'historique.
    let journal_rect = Rect::new(area.x, sky_rect.bottom(), area.width, JOURNAL_H);
    let mut scroll = 0;
    journal::render(buf, journal_rect, view.journal, ctx.now, &mut scroll);
    retitle(buf, journal_rect, " Journal ", "historique ▸ ");
    m.hits.add(journal_rect, Hit::Open(Tab::More, 0));

    // Supernova, ou une aide.
    let r = Rect::new(
        area.x + 1,
        journal_rect.bottom(),
        area.width.saturating_sub(2),
        BUTTON_H,
    );
    if r.bottom() > area.bottom() {
        return;
    }
    let n = snap.fragments_available;
    if n >= 1.0 {
        let pulse = 0.5 + 0.5 * (ctx.now * 3.0).sin();
        let label = format!(
            "✹ Supernova : +{} fragment{}",
            fmt::short(n),
            if n >= 2.0 { "s" } else { "" }
        );
        button(
            buf,
            &mut m.hits,
            m.press,
            r,
            "",
            &label,
            theme::PINK.glow(0.25 * pulse),
            Some(Hit::Ask(Confirm::Supernova)),
        );
    } else {
        let hint = if snap.hold_harvest {
            "Touche le ciel, ou maintiens le doigt pour récolter"
        } else {
            "Touche le ciel pour récolter ✦"
        };
        let t = draw::truncate(hint, r.width as usize);
        let x = r.x + r.width.saturating_sub(t.chars().count() as u16) / 2;
        draw::text(buf, x, r.y + 1, &t, theme::fg(theme::MUTED), r.right());
    }
}

// ----------------------------------------------------------------------
// Usines
// ----------------------------------------------------------------------

fn render_factories(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    vis: &mut Visuals,
    ui: &mut Ui,
    m: &mut MobileUi,
    scratch: &mut Vec<u64>,
) {
    let snap = ctx.snap;
    let now = ctx.now;
    // Sélecteur de quantité et achat automatique.
    let modes = [
        domain::BuyMode::One,
        domain::BuyMode::Ten,
        domain::BuyMode::TwentyFive,
        domain::BuyMode::Max,
    ];
    let auto_w = if snap.autobuy_unlocked { 11 } else { 0 };
    let mode_w = (area.width.saturating_sub(auto_w)) / 4;
    for (i, mode) in modes.iter().enumerate() {
        let r = Rect::new(area.x + i as u16 * mode_w, area.y, mode_w, BUTTON_H);
        let color = if snap.buy_mode == *mode {
            theme::GOLD
        } else {
            theme::BORDER
        };
        button(
            buf,
            &mut m.hits,
            m.press,
            r,
            "",
            mode.label(),
            color,
            Some(Hit::SetBuyMode(*mode)),
        );
    }
    if auto_w > 0 {
        let r = Rect::new(area.right() - auto_w, area.y, auto_w, BUTTON_H);
        let (label, color) = if snap.autobuy {
            ("Auto ●", theme::GREEN)
        } else {
            ("Auto ○", theme::GREY)
        };
        button(
            buf,
            &mut m.hits,
            m.press,
            r,
            "",
            label,
            color,
            Some(Hit::ToggleAutoBuy),
        );
    }

    // Production : une ligne, dépliable en courbe.
    let mut y = area.y + BUTTON_H;
    let prod_h = if m.production_open { 8 } else { 1 };
    let prod_rect = Rect::new(
        area.x,
        y,
        area.width,
        prod_h.min(area.bottom().saturating_sub(y)),
    );
    if m.production_open {
        sparkline::render(buf, prod_rect, ctx, scratch);
    } else {
        let text = format!(
            "▸ Production {}/s · {} générateurs",
            fmt::short(snap.production),
            snap.generators.iter().filter(|g| g.owned > 0).count()
        );
        draw::text(
            buf,
            area.x + 1,
            y,
            &draw::truncate(&text, area.width as usize - 2),
            theme::fg(theme::MUTED),
            area.right(),
        );
    }
    m.hits.add(prod_rect, Hit::ToggleProduction);
    y += prod_h;

    // Liste des générateurs.
    let list = Rect::new(area.x, y, area.width, area.bottom().saturating_sub(y));
    let count = snap.generators.len() as u16 + u16::from(snap.teaser.is_some());
    set_scroll_max(m, FACTORIES, count * CARD_H, list.height);
    let scroll = m.scroll[FACTORIES];
    for (i, g) in snap.generators.iter().enumerate() {
        let Some(r) = place(list, scroll, i as u16 * CARD_H, CARD_H) else {
            continue;
        };
        let Some(def) = ctx.catalog.generator(g.id) else {
            continue;
        };
        let gi = g.id.index();
        let color = theme::generator_color(gi);
        let flash = ((now - vis.gen_flash[gi.min(theme::GEN_SLOTS - 1)]) / 0.4).clamp(0.0, 1.0);
        let name_color = theme::TEXT.lerp(theme::WHITE, 1.0 - flash);
        let body = Rect::new(r.x, r.y, r.width.saturating_sub(BUY_W + 1), 3);
        ui.gen_rows[gi.min(theme::GEN_SLOTS - 1)] = Some(r.y + 1);

        let mut x = draw::text(
            buf,
            body.x + 1,
            r.y,
            &format!("{} ", def.symbol),
            theme::bold(color),
            body.right(),
        );
        x = draw::text(buf, x, r.y, def.name, theme::bold(name_color), body.right());
        draw::text(
            buf,
            x,
            r.y,
            &format!(" ×{}", g.owned),
            theme::fg(theme::MUTED),
            body.right(),
        );
        let stats = format!(
            "{}/s · {}% du total",
            fmt::short(g.production),
            (g.share * 100.0).round() as i64
        );
        draw::text(
            buf,
            body.x + 1,
            r.y + 1,
            &stats,
            theme::fg(theme::CYAN),
            body.right(),
        );
        // Barre vers le prochain palier (×2).
        let next = domain::generator::MILESTONES
            .iter()
            .find(|&&n| n > g.owned)
            .copied();
        match next {
            Some(n) => {
                let prev = domain::generator::MILESTONES
                    .iter()
                    .rev()
                    .find(|&&p| p <= g.owned)
                    .copied()
                    .unwrap_or(0);
                let label = format!(" ×2 à {n}");
                let bar_w = body.width.saturating_sub(label.chars().count() as u16 + 2);
                let k = (g.owned - prev) as f64 / (n - prev).max(1) as f64;
                let filled = (k * bar_w as f64).round() as u16;
                for i in 0..bar_w {
                    let (ch, c) = if i < filled {
                        ('━', color)
                    } else {
                        ('─', theme::BAR_EMPTY)
                    };
                    draw::put(buf, body, (body.x + 1 + i) as i32, (r.y + 2) as i32, ch, c);
                }
                draw::text(
                    buf,
                    body.x + 1 + bar_w,
                    r.y + 2,
                    &label,
                    theme::fg(theme::MUTED),
                    body.right(),
                );
            }
            None => {
                draw::text(
                    buf,
                    body.x + 1,
                    r.y + 2,
                    "tous les paliers atteints",
                    theme::fg(theme::GREEN),
                    body.right(),
                );
            }
        }
        m.hits.add(body, Hit::Details(Detail::Generator(g.id)));

        let buy = Rect::new(r.right() - BUY_W, r.y, BUY_W, 3);
        let bcolor = if g.affordable {
            theme::GOLD
        } else {
            theme::GREY
        };
        let qty = if g.quantity > 0 {
            format!("×{}", g.quantity)
        } else {
            snap.buy_mode.label().to_string()
        };
        button(
            buf,
            &mut m.hits,
            m.press,
            buy,
            &qty,
            &format!("{} ✦", fmt::short(g.cost)),
            bcolor,
            Some(Hit::BuyGenerator(g.id)),
        );
        separator(buf, r.x + 1, r.y + 3, r.width.saturating_sub(2));
    }
    if let Some(t) = snap.teaser {
        let vy = snap.generators.len() as u16 * CARD_H;
        if let Some(r) = place(list, scroll, vy, CARD_H) {
            let text = if t.particles_needed > 0 {
                format!("??? ─ à forger ({} particules)", t.particles_needed)
            } else if t.supernovas_needed > 0 {
                format!("??? ─ après {} Supernova(s)", t.supernovas_needed)
            } else {
                format!("??? ─ encore {} ✦ à récolter", fmt::short(t.remaining))
            };
            let pulse = 0.5 + 0.5 * (now * 1.5).sin();
            draw::text(
                buf,
                r.x + 1,
                r.y + 1,
                &draw::truncate(&text, r.width as usize - 2),
                theme::fg(theme::MUTED.lerp(theme::GREY, pulse)),
                r.right(),
            );
        }
    }
    scrollbar(buf, list, scroll, m.scroll_max[FACTORIES]);
}

// ----------------------------------------------------------------------
// Améliorer
// ----------------------------------------------------------------------

fn render_upgrades(buf: &mut Buffer, area: Rect, ctx: &Ctx, ui: &mut Ui, m: &mut MobileUi) {
    let snap = ctx.snap;
    let now = ctx.now;
    if snap.upgrades.is_empty() {
        let y = area.y + area.height / 3;
        let pulse = 0.5 + 0.5 * (now * 1.2).sin();
        for (i, l) in draw::wrap(upgrades::EMPTY_TEXT, area.width as usize - 4)
            .iter()
            .enumerate()
        {
            draw::text(
                buf,
                area.x + 2,
                y + i as u16,
                l,
                theme::fg(theme::MUTED.lerp(theme::GREY, pulse * 0.6)),
                area.right(),
            );
        }
        if let Some(next) = snap.next_upgrade {
            let name = ctx
                .catalog
                .upgrade(next.id)
                .map_or("?", |u| u.name.as_str());
            let text = format!(
                "Prochaine : {name} ({} / {})",
                fmt::short(next.current),
                fmt::short(next.needed)
            );
            for (i, l) in draw::wrap(&text, area.width as usize - 4)
                .iter()
                .enumerate()
            {
                draw::text(
                    buf,
                    area.x + 2,
                    y + 3 + i as u16,
                    l,
                    theme::fg(theme::CYAN),
                    area.right(),
                );
            }
        }
        return;
    }
    let affordable = snap.upgrades.iter().filter(|u| u.affordable).count();
    let r = Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), BUTTON_H);
    let (label, color, hit) = if affordable > 0 {
        (
            format!("Tout acheter ({affordable})"),
            theme::GOLD,
            Some(Hit::BuyAllUpgrades),
        )
    } else {
        (
            format!("{} améliorations proposées", snap.upgrades.len()),
            theme::GREY,
            None,
        )
    };
    button(buf, &mut m.hits, m.press, r, "", &label, color, hit);

    let list = Rect::new(
        area.x,
        area.y + BUTTON_H,
        area.width,
        area.height.saturating_sub(BUTTON_H),
    );
    set_scroll_max(
        m,
        UPGRADES,
        snap.upgrades.len() as u16 * CARD_H,
        list.height,
    );
    let scroll = m.scroll[UPGRADES];
    for (i, u) in snap.upgrades.iter().enumerate() {
        let Some(r) = place(list, scroll, i as u16 * CARD_H, CARD_H) else {
            continue;
        };
        let Some(def) = ctx.catalog.upgrade(u.id) else {
            continue;
        };
        ui.upg_rows.push((u.id, r.y + 1));
        let body = Rect::new(r.x, r.y, r.width.saturating_sub(BUY_W + 1), 3);
        draw::text(
            buf,
            body.x + 1,
            r.y,
            &draw::truncate(&def.name, body.width as usize - 1),
            theme::bold(theme::TEXT),
            body.right(),
        );
        let effect = match upgrades::impact(def.effect, snap) {
            Some(g) => format!("{} → {g}", touchify(&def.description)),
            None => touchify(&def.description),
        };
        for (k, l) in draw::wrap(&effect, body.width as usize - 1)
            .iter()
            .take(2)
            .enumerate()
        {
            draw::text(
                buf,
                body.x + 1,
                r.y + 1 + k as u16,
                l,
                theme::fg(theme::CYAN),
                body.right(),
            );
        }
        m.hits.add(body, Hit::Details(Detail::Upgrade(u.id)));
        let buy = Rect::new(r.right() - BUY_W, r.y, BUY_W, 3);
        let c = if u.affordable {
            theme::GOLD
        } else {
            theme::GREY
        };
        button(
            buf,
            &mut m.hits,
            m.press,
            buy,
            "",
            &format!("{} ✦", fmt::short(u.cost)),
            c,
            Some(Hit::BuyUpgrade(u.id)),
        );
        separator(buf, r.x + 1, r.y + 3, r.width.saturating_sub(2));
    }
    scrollbar(buf, list, scroll, m.scroll_max[UPGRADES]);
}

fn fragments_banner(buf: &mut Buffer, area: Rect, snap: &GameSnapshot) {
    let text = format!(
        "◆ {} libres sur {} · bonus +{}",
        fmt::short(snap.fragments_spendable),
        fmt::short(snap.fragments),
        fmt::percent(snap.fragment_bonus)
    );
    draw::text(
        buf,
        area.x + 1,
        area.y,
        &draw::truncate(&text, area.width as usize - 2),
        theme::bold(theme::CYAN),
        area.right(),
    );
}

fn render_atelier(buf: &mut Buffer, area: Rect, ctx: &Ctx, m: &mut MobileUi) {
    let snap = ctx.snap;
    fragments_banner(buf, area, snap);
    let list = Rect::new(
        area.x,
        area.y + 2,
        area.width,
        area.height.saturating_sub(2),
    );
    // Les améliorations acquises descendent en bas de liste.
    let mut order: Vec<usize> = (0..snap.perks.len()).collect();
    order.sort_by_key(|&i| snap.perks[i].owned);
    set_scroll_max(m, ATELIER, order.len() as u16 * CARD_H, list.height);
    let scroll = m.scroll[ATELIER];
    for (row, &i) in order.iter().enumerate() {
        let Some(r) = place(list, scroll, row as u16 * CARD_H, CARD_H) else {
            continue;
        };
        let p = snap.perks[i];
        let Some(def) = ctx.catalog.perk(p.id) else {
            continue;
        };
        let body = Rect::new(r.x, r.y, r.width.saturating_sub(BUY_W + 1), 3);
        let color = if p.owned {
            theme::GREEN
        } else if p.affordable {
            theme::GOLD
        } else {
            theme::GREY
        };
        let x = draw::text(
            buf,
            body.x + 1,
            r.y,
            &format!("{} ", def.symbol),
            theme::bold(color),
            body.right(),
        );
        let name_style = if p.owned {
            theme::fg(theme::GREEN)
        } else {
            theme::bold(theme::TEXT)
        };
        draw::text(buf, x, r.y, def.name, name_style, body.right());
        for (k, l) in draw::wrap(&touchify(def.description), body.width as usize - 1)
            .iter()
            .take(2)
            .enumerate()
        {
            draw::text(
                buf,
                body.x + 1,
                r.y + 1 + k as u16,
                l,
                theme::fg(theme::MUTED),
                body.right(),
            );
        }
        m.hits.add(body, Hit::Details(Detail::Perk(i)));
        let buy = Rect::new(r.right() - BUY_W, r.y, BUY_W, 3);
        if p.owned {
            button(
                buf,
                &mut m.hits,
                m.press,
                buy,
                "",
                "✓ acquise",
                theme::GREEN,
                None,
            );
        } else {
            button(
                buf,
                &mut m.hits,
                m.press,
                buy,
                "",
                &format!("{} ◆", fmt::short(def.cost)),
                color,
                Some(Hit::BuyPerk(p.id)),
            );
        }
        separator(buf, r.x + 1, r.y + 3, r.width.saturating_sub(2));
    }
    scrollbar(buf, list, scroll, m.scroll_max[ATELIER]);
}

fn render_constellations(buf: &mut Buffer, area: Rect, ctx: &Ctx, m: &mut MobileUi) {
    let snap = ctx.snap;
    fragments_banner(buf, area, snap);
    let list = Rect::new(
        area.x,
        area.y + 2,
        area.width,
        area.height.saturating_sub(2),
    );
    set_scroll_max(
        m,
        CONSTELLATIONS,
        snap.constellations.len() as u16 * CARD_H,
        list.height,
    );
    let scroll = m.scroll[CONSTELLATIONS];
    for (i, c) in snap.constellations.iter().enumerate() {
        let Some(r) = place(list, scroll, i as u16 * CARD_H, CARD_H) else {
            continue;
        };
        let Some(def) = ctx.catalog.constellation(c.id) else {
            continue;
        };
        let body = Rect::new(r.x, r.y, r.width.saturating_sub(BUY_W + 1), 3);
        let color = if c.maxed {
            theme::GREEN
        } else if c.affordable {
            theme::GOLD
        } else {
            theme::GREY
        };
        let x = draw::text(
            buf,
            body.x + 1,
            r.y,
            &format!("{} ", def.symbol),
            theme::bold(color),
            body.right(),
        );
        let x = draw::text(
            buf,
            x,
            r.y,
            def.name,
            theme::bold(theme::TEXT),
            body.right(),
        );
        draw::text(
            buf,
            x,
            r.y,
            &format!(" · niv. {}", c.level),
            theme::fg(theme::MUTED),
            body.right(),
        );
        let now_txt = if c.level > 0 {
            shop::constellation_total(def.effect, c.level)
        } else {
            "pas encore active".to_string()
        };
        draw::text(
            buf,
            body.x + 1,
            r.y + 1,
            &draw::truncate(&now_txt, body.width as usize - 1),
            theme::fg(theme::CYAN),
            body.right(),
        );
        if !c.maxed {
            let next = format!("→ {}", shop::constellation_total(def.effect, c.level + 1));
            draw::text(
                buf,
                body.x + 1,
                r.y + 2,
                &draw::truncate(&next, body.width as usize - 1),
                theme::fg(theme::MUTED),
                body.right(),
            );
        }
        m.hits.add(body, Hit::Details(Detail::Constellation(i)));
        let buy = Rect::new(r.right() - BUY_W, r.y, BUY_W, 3);
        if c.maxed {
            button(
                buf,
                &mut m.hits,
                m.press,
                buy,
                "",
                "MAX",
                theme::GREEN,
                None,
            );
        } else {
            button(
                buf,
                &mut m.hits,
                m.press,
                buy,
                "niv. +1",
                &format!("{} ◆", fmt::short(c.cost)),
                color,
                Some(Hit::BuyConstellation(c.id)),
            );
        }
        separator(buf, r.x + 1, r.y + 3, r.width.saturating_sub(2));
    }
    scrollbar(buf, list, scroll, m.scroll_max[CONSTELLATIONS]);
}

// ----------------------------------------------------------------------
// Cosmos : un menu des fonctionnalités débloquées
// ----------------------------------------------------------------------

struct CosmosEntry {
    symbol: char,
    title: &'static str,
    status: String,
    color: Rgb,
    hit: Hit,
}

fn cosmos_entries(ctx: &Ctx) -> Vec<CosmosEntry> {
    let s = ctx.snap;
    let mut v = Vec::new();
    let mut add = |symbol, title, status: String, color, hit| {
        v.push(CosmosEntry {
            symbol,
            title,
            status,
            color,
            hit,
        })
    };
    if let Some((_, left)) = s.pending_event {
        add(
            '✹',
            "Événement cosmique",
            format!("à décider · {}", fmt::countdown(left)),
            theme::PINK,
            Hit::OpenPage(Page::Event),
        );
    }
    if s.crunch_available {
        add(
            '●',
            "Big Crunch",
            "prêt".to_string(),
            theme::PINK,
            if s.universes_unlocked {
                Hit::Key(Intent::BigCrunch)
            } else {
                Hit::Ask(Confirm::Crunch)
            },
        );
    }
    if let Some(d) = s.daily {
        add(
            '✧',
            "Défi du jour",
            format!("en cours · {}", fmt::countdown(d.left)),
            theme::GOLD,
            Hit::OpenPage(Page::Daily),
        );
    } else if s.daily_unlocked {
        add(
            '✧',
            "Défi du jour",
            "disponible".to_string(),
            theme::GOLD,
            Hit::OpenPage(Page::Daily),
        );
    }
    if s.challenges.iter().any(|c| c.unlocked) {
        let done = s.challenges.iter().filter(|c| c.done).count();
        let status = match s.active_challenge {
            Some(a) => format!("en cours · {}", fmt::percent(a.progress)),
            None => format!("{done}/{} réussis", s.challenges.len()),
        };
        add(
            '◆',
            "Défis",
            status,
            theme::PINK,
            Hit::OpenPage(Page::Challenges),
        );
    }
    if s.lab_unlocked {
        let status = match s.research {
            Some(r) => format!("recherche · {}", fmt::duration(r.remaining)),
            None => format!("{} ✶ libres", s.dark_matter_spendable),
        };
        add(
            '✶',
            "Laboratoire",
            status,
            theme::CYAN,
            Hit::OpenPage(Page::Lab),
        );
        add(
            '◎',
            "Cosmogonie",
            format!("{} Big Bang", s.big_bangs),
            theme::GOLD,
            Hit::OpenPage(Page::Cosmogony),
        );
    }
    if s.universes_unlocked {
        let keys = s.universes.iter().filter(|u| u.key).count();
        add(
            '◉',
            "Univers parallèles",
            format!("clés {keys}/{}", s.universes.len().saturating_sub(1)),
            theme::CYAN,
            Hit::Key(Intent::Universes),
        );
    }
    if s.starmap_unlocked {
        add(
            '◈',
            "Carte stellaire",
            format!(
                "explorés {} · flottes {}/{}",
                s.explored,
                s.fleets.len(),
                s.fleets_max
            ),
            theme::CYAN,
            Hit::OpenPage(Page::StarMap),
        );
        add(
            '◇',
            "Reliques",
            format!("{}/{} équipées", s.relics_equipped, s.relic_slots),
            theme::GOLD,
            Hit::OpenPage(Page::Relics),
        );
    }
    if s.megastructures_unlocked {
        add(
            '▣',
            "Mégastructures",
            format!("{} ◈", fmt::short(s.exotic)),
            theme::GOLD,
            Hit::OpenPage(Page::Mega),
        );
    }
    v
}

fn render_cosmos(buf: &mut Buffer, area: Rect, ctx: &Ctx, m: &mut MobileUi) {
    let entries = cosmos_entries(ctx);
    if entries.is_empty() {
        let text = "Déclenche ta première Supernova : le Cosmos s'ouvrira ici.";
        for (i, l) in draw::wrap(text, area.width as usize - 4).iter().enumerate() {
            draw::text(
                buf,
                area.x + 2,
                area.y + area.height / 3 + i as u16,
                l,
                theme::fg(theme::MUTED),
                area.right(),
            );
        }
        return;
    }
    set_scroll_max(m, COSMOS, entries.len() as u16 * CARD_H, area.height);
    let scroll = m.scroll[COSMOS];
    for (i, e) in entries.iter().enumerate() {
        let Some(r) = place(area, scroll, i as u16 * CARD_H, CARD_H) else {
            continue;
        };
        let card = Rect::new(r.x + 1, r.y, r.width.saturating_sub(2), BUTTON_H);
        let pressed = m.press.is_some_and(|p| p.rect == card);
        if pressed {
            draw::fill_bg(buf, card, e.color.lerp(theme::BLACK, 0.6));
        }
        draw::card(buf, card, card, e.color);
        let x = draw::text(
            buf,
            card.x + 2,
            card.y + 1,
            &format!("{} ", e.symbol),
            theme::bold(e.color),
            card.right() - 1,
        );
        let st_x = draw::text_right(
            buf,
            card.right() - 2,
            card.y + 1,
            &e.status,
            theme::fg(theme::MUTED),
            x + 1,
        );
        draw::text(
            buf,
            x,
            card.y + 1,
            e.title,
            theme::bold(theme::TEXT),
            st_x.saturating_sub(1),
        );
        m.hits.add(card, e.hit);
    }
    scrollbar(buf, area, scroll, m.scroll_max[COSMOS]);
}

// ----------------------------------------------------------------------
// Plus : Infos, Options, Sauvegarde
// ----------------------------------------------------------------------

fn render_infos(buf: &mut Buffer, area: Rect, ctx: &Ctx, m: &mut MobileUi) {
    let width = area.width.saturating_sub(3) as usize;
    let mut lines: Vec<(String, Style)> = Vec::new();
    for (text, style) in codex::lines(ctx) {
        if text.is_empty() {
            lines.push((String::new(), style));
            continue;
        }
        for l in draw::wrap(&touchify(&text), width) {
            lines.push((l, style));
        }
    }
    set_scroll_max(m, INFOS, lines.len() as u16, area.height);
    let scroll = m.scroll[INFOS];
    for (i, (l, style)) in lines
        .iter()
        .skip(scroll as usize)
        .take(area.height as usize)
        .enumerate()
    {
        draw::text(
            buf,
            area.x + 1,
            area.y + i as u16,
            l,
            *style,
            area.right() - 1,
        );
    }
    scrollbar(buf, area, scroll, m.scroll_max[INFOS]);
}

#[allow(clippy::too_many_arguments)]
fn option_row(
    buf: &mut Buffer,
    m: &mut MobileUi,
    r: Rect,
    title: &str,
    value: &str,
    action: &str,
    color: Rgb,
    hit: Hit,
) {
    let body = Rect::new(r.x, r.y, r.width.saturating_sub(BUY_W + 1), 3);
    draw::text(
        buf,
        body.x + 1,
        r.y,
        title,
        theme::bold(theme::TEXT),
        body.right(),
    );
    draw::text(
        buf,
        body.x + 1,
        r.y + 1,
        &draw::truncate(value, body.width as usize - 1),
        theme::fg(theme::CYAN),
        body.right(),
    );
    let b = Rect::new(r.right() - BUY_W, r.y, BUY_W, 3);
    button(buf, &mut m.hits, m.press, b, "", action, color, Some(hit));
    separator(buf, r.x + 1, r.y + 3, r.width.saturating_sub(2));
}

fn render_options(buf: &mut Buffer, area: Rect, ctx: &Ctx, m: &mut MobileUi) {
    let snap = ctx.snap;
    let rows: [(String, String, String, Rgb, Hit); 4] = [
        (
            "Couleurs".into(),
            crate::fx::palette::mode_name(snap.color_mode).to_string(),
            "Changer".into(),
            theme::FOCUS,
            Hit::CycleColors,
        ),
        (
            "Images par seconde".into(),
            format!("{} images/s", snap.frame_rate),
            "Changer".into(),
            theme::FOCUS,
            Hit::CycleFrameRate,
        ),
        (
            "Effets réduits".into(),
            if snap.reduced_effects {
                "activés (économise la batterie)".into()
            } else {
                "désactivés".into()
            },
            if snap.reduced_effects {
                "Désactiver".into()
            } else {
                "Activer".into()
            },
            theme::FOCUS,
            Hit::ToggleReducedEffects,
        ),
        (
            "Recommencer une partie".into(),
            "Efface la progression".into(),
            "Recommencer".into(),
            theme::GREY,
            Hit::Ask(Confirm::NewGame),
        ),
    ];
    set_scroll_max(m, OPTIONS, rows.len() as u16 * CARD_H, area.height);
    let scroll = m.scroll[OPTIONS];
    for (i, (title, value, action, color, hit)) in rows.iter().enumerate() {
        if let Some(r) = place(area, scroll, i as u16 * CARD_H, CARD_H) {
            option_row(buf, m, r, title, value, action, *color, *hit);
        }
    }
}

fn render_save(buf: &mut Buffer, area: Rect, m: &mut MobileUi, version: &str) {
    let text = "Ta sauvegarde est sur ce téléphone. Exporte-la pour la garder, \
                ou pour la reprendre sur ordinateur (fonderie-stellaire --import).";
    let mut y = area.y + 1;
    for l in draw::wrap(text, area.width as usize - 4) {
        draw::text(
            buf,
            area.x + 2,
            y,
            &l,
            theme::fg(theme::MUTED),
            area.right(),
        );
        y += 1;
    }
    y += 1;
    let w = area.width.saturating_sub(2);
    for (label, req, color) in [
        ("Exporter la sauvegarde", Request::ExportSave, theme::GOLD),
        ("Importer une sauvegarde", Request::ImportSave, theme::FOCUS),
    ] {
        if y + BUTTON_H > area.bottom() {
            break;
        }
        let r = Rect::new(area.x + 1, y, w, BUTTON_H);
        button(
            buf,
            &mut m.hits,
            m.press,
            r,
            "",
            label,
            color,
            Some(Hit::Request(req)),
        );
        y += BUTTON_H + 1;
    }
    let v = format!("Version {version}");
    draw::text(
        buf,
        area.x + 2,
        area.bottom().saturating_sub(1),
        &v,
        theme::fg(theme::GREY),
        area.right(),
    );
}

// ----------------------------------------------------------------------
// Fiche détail
// ----------------------------------------------------------------------

fn detail_lines(ctx: &Ctx, d: Detail, width: usize) -> (String, Vec<(String, Style)>) {
    match d {
        Detail::Relic(i) => {
            let name = ctx.catalog.relics.get(i).map(|r| r.name).unwrap_or("?");
            let shown = if ctx.snap.relics.get(i).is_some_and(|v| v.rarity.is_some()) {
                name
            } else {
                "Relique inconnue"
            };
            (
                shown.to_string(),
                super::cosmos::relic_sheet(ctx, i, width).0,
            )
        }
        Detail::Generator(id) => {
            let name = ctx.catalog.generator(id).map(|g| g.name).unwrap_or("?");
            let lines = ctx
                .snap
                .generators
                .iter()
                .find(|g| g.id == id)
                .map(|g| generators::detail_lines(ctx, g, width))
                .unwrap_or_default();
            (name.to_string(), lines)
        }
        Detail::Upgrade(id) => (
            "Amélioration".to_string(),
            upgrades::detail_lines(ctx, id, width)
                .into_iter()
                .map(|(l, s)| (touchify(&l), s))
                .collect(),
        ),
        Detail::Perk(i) => {
            let name = ctx
                .snap
                .perks
                .get(i)
                .and_then(|p| ctx.catalog.perk(p.id))
                .map(|p| p.name)
                .unwrap_or("?");
            let (desc, hint) = shop::detail(ctx, ShopItem::Perk(i));
            (name.to_string(), wrap_pair(&desc, &hint, width))
        }
        Detail::Constellation(i) => {
            let name = ctx
                .snap
                .constellations
                .get(i)
                .and_then(|c| ctx.catalog.constellation(c.id))
                .map(|c| c.name)
                .unwrap_or("?");
            let (desc, hint) = shop::detail(ctx, ShopItem::Constellation(i));
            (name.to_string(), wrap_pair(&desc, &hint, width))
        }
    }
}

fn wrap_pair(desc: &str, hint: &str, width: usize) -> Vec<(String, Style)> {
    let hint = hint
        .replace(
            "[Entrée] pour l'acheter.",
            "Touche le bouton pour l'acheter.",
        )
        .replace("[Entrée] →", "Prochain niveau →")
        .replace(" [P]", "");
    let mut out: Vec<(String, Style)> = draw::wrap(&touchify(desc), width)
        .into_iter()
        .map(|l| (l, theme::fg(theme::TEXT)))
        .collect();
    out.push((String::new(), Style::default()));
    out.extend(
        draw::wrap(&hint, width)
            .into_iter()
            .map(|l| (l, theme::fg(theme::CYAN))),
    );
    out
}

fn render_detail(buf: &mut Buffer, area: Rect, ctx: &Ctx, d: Detail, m: &mut MobileUi) {
    let width = area.width.saturating_sub(4) as usize;
    let (title, lines) = detail_lines(ctx, d, width);
    let action = match d {
        Detail::Relic(i) => super::cosmos::relic_sheet(ctx, i, width).1,
        _ => None,
    };
    let extra = if action.is_some() { BUTTON_H + 1 } else { 0 };
    let h = (lines.len() as u16 + 4 + extra).min(area.height);
    let r = Rect::new(area.x, area.bottom() - h, area.width, h);
    draw::clear(buf, r);
    draw::fill_bg(buf, r, theme::SELECT_BG);
    draw::card(buf, r, r, theme::FOCUS);
    let t = format!(" {title} ");
    draw::text(
        buf,
        r.x + 2,
        r.y,
        &t,
        theme::bold(theme::FOCUS),
        r.right() - 1,
    );
    for (i, (l, style)) in lines.iter().enumerate() {
        let y = r.y + 1 + i as u16;
        if y + 2 + extra >= r.bottom() {
            break;
        }
        draw::text(buf, r.x + 2, y, l, *style, r.right() - 1);
    }
    let close = "toucher pour fermer";
    draw::text_right(
        buf,
        r.right() - 2,
        r.bottom() - 1,
        close,
        theme::fg(theme::MUTED),
        r.x,
    );
    // Fermer d'un toucher n'importe où, sauf sur le bouton de la fiche.
    m.hits.add(area, Hit::CloseDetail);
    if let Some((label, color, hit)) = action {
        let b = Rect::new(
            r.x + 2,
            r.bottom() - 1 - BUTTON_H,
            r.width.saturating_sub(4),
            BUTTON_H,
        );
        button(buf, &mut m.hits, m.press, b, "", &label, color, Some(hit));
    }
}

// ----------------------------------------------------------------------
// Écrans hérités du terminal : rangée de boutons tactiles
// ----------------------------------------------------------------------

/// Boutons qui pilotent une popup du terminal (affichée en grille dense).
pub fn render_controls(buf: &mut Buffer, area: Rect, ui: &Ui, m: &mut MobileUi) {
    m.hits.clear();
    // En grille dense, les cellules sont deux fois plus petites : boutons plus hauts.
    let h = 5u16.min(area.height);
    let mut rows: Vec<Vec<(String, Hit, Rgb)>> = Vec::new();
    let key =
        |label: &str, intent: Intent, color: Rgb| (label.to_string(), Hit::Key(intent), color);
    let arrows = vec![
        key("◀", Intent::PrevPanel, theme::BORDER),
        key("▲", Intent::Up, theme::BORDER),
        key("▼", Intent::Down, theme::BORDER),
        key("▶", Intent::NextPanel, theme::BORDER),
    ];
    if let Some(p) = ui.portal {
        let mut r = vec![key("◀", Intent::PrevPanel, theme::BORDER)];
        if p.crunch {
            r.push(key("Choisir cet univers", Intent::Buy, theme::PINK));
        } else {
            r.push(key("Fermer", Intent::Escape, theme::GREY));
        }
        r.push(key("▶", Intent::NextPanel, theme::BORDER));
        rows.push(r);
    } else if ui.popup.is_some() {
        rows.push(vec![
            key("Non", Intent::No, theme::GREY),
            key("Oui", Intent::Yes, theme::PINK),
        ]);
    } else if ui.event.is_some() {
        rows.push(vec![
            key("▲", Intent::Up, theme::BORDER),
            key("▼", Intent::Down, theme::BORDER),
            key("Plus tard", Intent::Escape, theme::GREY),
            key("Choisir", Intent::Buy, theme::GOLD),
        ]);
    } else if ui.mega.is_some() {
        let mut structures = Vec::new();
        for i in 0..6 {
            structures.push(key(&format!("{}", i + 1), Intent::QuickBuy(i), theme::GOLD));
        }
        structures.push(key("Démolir", Intent::Challenges, theme::RED));
        rows.push(structures);
        let mut r = arrows.clone();
        r.push(key("OK", Intent::Buy, theme::GOLD));
        r.push(key("✕", Intent::Escape, theme::GREY));
        rows.push(r);
    } else if ui.starmap.is_some() {
        let mut r = arrows.clone();
        r.push(key("Envoyer", Intent::Buy, theme::GOLD));
        r.push(key("Auto", Intent::BuyAll, theme::CYAN));
        r.push(key("✕", Intent::Escape, theme::GREY));
        rows.push(r);
    } else {
        let mut r = arrows;
        r.push(key("OK", Intent::Buy, theme::GOLD));
        r.push(key("✕", Intent::Escape, theme::GREY));
        rows.push(r);
    }
    let total_h = h * rows.len() as u16;
    let top = area.bottom().saturating_sub(total_h);
    let strip = Rect::new(area.x, top, area.width, total_h);
    draw::clear(buf, strip);
    draw::fill_bg(buf, strip, theme::BLACK);
    for (ri, row) in rows.iter().enumerate() {
        let y = top + ri as u16 * h;
        let n = row.len() as u16;
        let w = area.width / n.max(1);
        for (i, (label, hit, color)) in row.iter().enumerate() {
            let x = area.x + i as u16 * w;
            let width = if i as u16 == n - 1 {
                area.right() - x
            } else {
                w
            };
            let r = Rect::new(x, y, width, h);
            button(buf, &mut m.hits, m.press, r, "", label, *color, Some(*hit));
        }
    }
}
