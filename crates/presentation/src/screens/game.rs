//! Écran de jeu : compose les widgets puis applique les post-effets
//! (particules, bordures animées, cascade d'apparition, toasts, popup).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::draw;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::fx::Rgb;
use crate::layout::{game_layout, GameLayout};
use crate::nav::ScreenId;
use crate::theme;
use crate::ui::{Panel, Ui};
use crate::visuals::Visuals;
use crate::widgets::frame::Frame;
use crate::widgets::journal::Journal;
use crate::widgets::toast::Toasts;
use crate::widgets::{
    ceremony, challenges, codex, cosmos, daily, event, generators, header, help_bar, journal, lab,
    mega, modal, options, portals, relics, shop, sky, sparkline, starmap, upgrades, Ctx,
};

/// Sous cette hauteur, un onglet ouvert réduit l'en-tête à une ligne.
pub const TAB_COMPACT_BELOW: u16 = 30;

/// Décalage entre deux panneaux de la cascade.
pub const CASCADE_STAGGER: f64 = 0.08;
pub const CASCADE_DURATION: f64 = 0.35;

pub struct GameView<'a> {
    /// Jour (depuis 1970), pour le Défi du jour.
    pub today: u64,
    pub save_path: &'a str,
    pub version: &'a str,
    pub sync: &'a crate::widgets::sync::SyncView,
    pub leaderboard: &'a crate::widgets::leaderboard::LeaderboardView,
    pub journal: &'a Journal,
    pub toasts: &'a Toasts,
    pub spark_scratch: &'a mut Vec<u64>,
    /// Écran de la barre ouvert, et celui qu'ouvrirait `Tab`.
    pub screen: crate::nav::ScreenId,
    pub next_screen: crate::nav::ScreenId,
}

pub fn render(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    vis: &mut Visuals,
    ui: &mut Ui,
    view: GameView,
) {
    // Le panneau des améliorations s'agrandit (en douceur) quand il a le focus.
    let t = ((ctx.now - ui.focus_changed_at) / 0.3).clamp(0.0, 1.0);
    let k = crate::fx::easing::ease_in_out_sine(t);
    let expand = if ui.focus == Panel::Upgrades {
        k
    } else {
        1.0 - k
    };
    let layout = game_layout(area, expand);
    ui.layout = layout;

    // Onglet (ordinateur) : l'écran ouvert remplace les panneaux de la
    // fonderie, entre l'en-tête et la barre d'aide.
    let tab = ctx.frame == Frame::Tab && view.screen != ScreenId::Foundry;
    // Terminal bas : l'en-tête se réduit à une ligne pour laisser à l'onglet
    // la hauteur d'une fenêtre plein écran.
    let compact = tab && area.height < TAB_COMPACT_BELOW;
    let screen_area = if compact {
        Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        )
    } else if tab {
        layout.body
    } else {
        area
    };

    if compact {
        compact_header(
            buf,
            Rect::new(area.x, area.y, area.width, 1),
            ctx,
            view.screen,
        );
    } else {
        header::render(buf, layout.header, ctx, vis);
        crate::widgets::nav::render_tabs(buf, layout.header, ctx.snap, view.screen);
    }
    if !tab {
        generators::render(buf, layout.generators, ctx, vis, ui);
        sky::render(buf, layout.sky, ctx, vis);
        upgrades::render(buf, layout.upgrades, ctx, vis, ui);
        sparkline::render(buf, layout.production, ctx, view.spark_scratch);
        journal::render(
            buf,
            layout.journal,
            view.journal,
            ctx.now,
            &mut ui.journal_scroll,
        );
    }
    if !compact {
        help_bar::render(
            buf,
            layout.help,
            ctx.snap,
            Some((view.screen, view.next_screen)),
            &mut vis.help_seen,
            vis.mode_flash,
            ctx.now,
        );
    }

    if !tab {
        vis.particles.draw(buf, area);
        animate_borders(buf, &layout, ctx, ui);
        cascade(buf, &layout, ctx.now, vis.cascade_start);
    }

    view.toasts.render(buf, area, layout.sky.y, ctx.now);

    // Cérémonie de Clé d'univers, par-dessus le jeu.
    if let Some((start, universe)) = vis.key_ceremony {
        if let Some(def) = ctx.catalog.universe(universe) {
            ceremony::render(buf, area, ctx.now, start, def);
        }
    }

    if let Some(popup) = ui.popup {
        if ui.popup_crunch {
            modal::render_crunch(buf, area, ctx, &popup);
        } else {
            modal::render(buf, area, ctx, &popup);
        }
    }
    if let Some(popup) = ui.lab {
        lab::render(buf, screen_area, ctx, &popup, ui.lab_sel);
    }
    if let Some(popup) = ui.cosmos {
        cosmos::render(
            buf,
            screen_area,
            ctx,
            &popup,
            ui.cosmos_sel,
            ui.cosmos_armed,
        );
    }
    if let Some(popup) = ui.options.filter(|_| ui.options_sync) {
        crate::widgets::sync::render(buf, screen_area, ctx, &popup, ui.options_armed, view.sync);
    } else if let Some(popup) = ui.options {
        let leaderboard = view.leaderboard.short();
        let info = options::OptionsInfo {
            save_path: view.save_path,
            version: view.version,
            sync: view.sync.short(),
            leaderboard: &leaderboard,
        };
        options::render(
            buf,
            screen_area,
            ctx,
            &popup,
            ui.options_sel,
            ui.options_armed,
            &info,
        );
    }
    if let Some(popup) = ui.leaderboard {
        crate::widgets::leaderboard::render(buf, screen_area, ctx, &popup, view.leaderboard);
    }
    if let Some(popup) = ui.daily {
        daily::render(buf, screen_area, ctx, &popup, view.today);
    }
    if let Some(popup) = ui.mega {
        mega::render(buf, screen_area, ctx, &popup, ui.mega_sel, vis.mega_pop);
    }
    if let Some(popup) = ui.relics {
        relics::render(buf, screen_area, ctx, &popup, ui.relics_sel);
    }
    if let Some(m) = ui.starmap {
        starmap::render(buf, screen_area, ctx, &m, &vis.stars, &vis.map_pops);
    }
    if let Some(popup) = ui.event {
        event::render(buf, area, ctx, &popup);
    }
    if let Some(r) = vis.daily_result.as_ref() {
        daily::render_result(buf, area, ctx.now, r);
    }
    if let Some(at) = vis.iris_at {
        daily::render_iris(buf, area, ctx.now, at);
    }
    // Relique trouvée : par-dessus tout, même la carte.
    if let Some(card) = vis.relic_card {
        relics::render_card(buf, area, ctx, &card);
    }
    if let Some(at) = ui.wipe_at {
        options::render_wipe(buf, area, ctx.now, at);
    }
    if let Some(popup) = ui.challenges {
        challenges::render(
            buf,
            screen_area,
            ctx,
            &popup,
            ui.challenges_sel,
            ui.challenge_armed,
        );
    }
    if let Some(shop) = ui.shop {
        shop::render(buf, screen_area, ctx, &shop, ui.shop_sel);
    }
    if let Some(codex) = ui.codex {
        codex::render(buf, screen_area, ctx, &codex, &mut ui.codex_scroll);
    }
    // Les portails recouvrent tout l'écran.
    if let Some(portal) = ui.portal {
        portals::render(buf, area, ctx, &portal, &vis.stars);
    }
    // Menu central et aide, par-dessus tout.
    if let Some(menu) = ui.menu {
        crate::widgets::nav::render_menu(buf, area, ctx, &menu, ui.menu_sel);
    }
    if let Some(help) = ui.help {
        crate::widgets::nav::render_help(buf, area, ctx, &help, &mut ui.help_scroll);
    }
}

/// Bordure du panneau actif : une lueur parcourt le contour. Pendant la tempête
/// solaire, toutes les bordures pulsent en orange.
fn animate_borders(buf: &mut Buffer, layout: &GameLayout, ctx: &Ctx, ui: &Ui) {
    let now = ctx.now;
    let focused = match ui.focus {
        Panel::Generators => layout.generators,
        Panel::Upgrades => layout.upgrades,
    };
    let n = effects::perimeter_len(focused) as f64;
    if n > 0.0 {
        let head = (now * 28.0) % n;
        // Petit éclat juste après un changement de focus.
        let burst = (1.0 - (now - ui.focus_changed_at) / 0.4).clamp(0.0, 1.0);
        effects::for_each_border_cell(buf, focused, |i, cell| {
            if !draw::is_box_char(cell.symbol()) {
                return;
            }
            let d = (i as f64 - head).abs();
            let d = d.min(n - d);
            let k = (-(d / 5.0).powi(2)).exp();
            let c = theme::FOCUS.lerp(theme::WHITE, (k * 0.85 + burst * 0.6).min(1.0));
            cell.set_fg(c.color());
        });
    }

    if ctx.snap.storm_remaining > 0.0 {
        let pulse = 0.5 + 0.5 * (now * 5.0).sin();
        let amount = 0.3 + 0.5 * pulse;
        for rect in [
            layout.header,
            layout.generators,
            layout.sky,
            layout.upgrades,
            layout.production,
            layout.journal,
        ] {
            effects::for_each_border_cell(buf, rect, |_, cell| {
                if draw::is_box_char(cell.symbol()) {
                    let base = Rgb::from_color(cell.fg).unwrap_or(theme::BORDER);
                    cell.set_fg(base.lerp(theme::ORANGE, amount).color());
                }
            });
        }
    }
}

/// Cascade d'apparition : chaque panneau glisse et apparaît avec un décalage.
fn cascade(buf: &mut Buffer, layout: &GameLayout, now: f64, start: f64) {
    for (i, rect) in layout.panels().iter().enumerate() {
        let p = (now - start - i as f64 * CASCADE_STAGGER) / CASCADE_DURATION;
        if p >= 1.0 {
            continue;
        }
        if p <= 0.0 {
            draw::clear(buf, *rect);
            continue;
        }
        let e = ease_out_cubic(p);
        let dy = ((1.0 - e) * 2.0).round() as i32;
        let dx = if i % 2 == 0 { -1 } else { 1 } * ((1.0 - e) * 4.0).round() as i32;
        effects::offset(buf, *rect, dx, dy);
        effects::fade(buf, *rect, 1.0 - e);
    }
}

/// En-tête d'une ligne (onglet ouvert sur un terminal bas) : les onglets à
/// gauche, la poussière et la production à droite.
fn compact_header(buf: &mut Buffer, line: Rect, ctx: &Ctx, screen: ScreenId) {
    let snap = ctx.snap;
    let border = theme::fg(theme::BORDER);
    for x in line.left()..line.right() {
        buf[(x, line.y)].set_symbol("─").set_style(border);
    }
    let stats = format!(
        " ✦ {} · +{} /s ",
        crate::fmt::short(snap.stardust),
        crate::fmt::short(snap.production)
    );
    let stats_w = stats.chars().count() as u16;
    let stats_x = line.right().saturating_sub(stats_w + 1);
    draw::text(
        buf,
        stats_x,
        line.y,
        &stats,
        theme::bold(theme::GOLD),
        line.right(),
    );
    let tabs = Rect::new(line.x, line.y, stats_x.saturating_sub(line.x), 1);
    crate::widgets::nav::render_tabs_on(buf, tabs, snap, screen);
}
