//! Écran de jeu : compose les widgets puis applique les post-effets
//! (particules, bordures animées, cascade d'apparition, toasts, popup).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::draw;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::fx::Rgb;
use crate::layout::{game_layout, GameLayout};
use crate::theme;
use crate::ui::{Panel, Ui};
use crate::visuals::Visuals;
use crate::widgets::journal::Journal;
use crate::widgets::toast::Toasts;
use crate::widgets::{
    ceremony, challenges, codex, cosmos, generators, header, help_bar, journal, lab, modal,
    options, portals, shop, sky, sparkline, starmap, upgrades, Ctx,
};

/// Décalage entre deux panneaux de la cascade.
pub const CASCADE_STAGGER: f64 = 0.08;
pub const CASCADE_DURATION: f64 = 0.35;

pub struct GameView<'a> {
    pub save_path: &'a str,
    pub version: &'a str,
    pub journal: &'a Journal,
    pub toasts: &'a Toasts,
    pub spark_scratch: &'a mut Vec<u64>,
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

    header::render(buf, layout.header, ctx, vis);
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
    help_bar::render(
        buf,
        layout.help,
        ctx.snap.buy_mode,
        ctx.snap.hold_harvest,
        vis.mode_flash,
        ctx.now,
    );

    vis.particles.draw(buf, area);

    animate_borders(buf, &layout, ctx, ui);
    cascade(buf, &layout, ctx.now, vis.cascade_start);

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
        lab::render(buf, area, ctx, &popup, ui.lab_sel);
    }
    if let Some(popup) = ui.cosmos {
        cosmos::render(buf, area, ctx, &popup, ui.cosmos_sel, ui.cosmos_armed);
    }
    if let Some(popup) = ui.options {
        let info = options::OptionsInfo {
            save_path: view.save_path,
            version: view.version,
        };
        options::render(
            buf,
            area,
            ctx,
            &popup,
            ui.options_sel,
            ui.options_armed,
            &info,
        );
    }
    if let Some(m) = ui.starmap {
        starmap::render(buf, area, ctx, &m, &vis.stars, &vis.map_pops);
    }
    if let Some(at) = ui.wipe_at {
        options::render_wipe(buf, area, ctx.now, at);
    }
    if let Some(popup) = ui.challenges {
        challenges::render(
            buf,
            area,
            ctx,
            &popup,
            ui.challenges_sel,
            ui.challenge_armed,
        );
    }
    if let Some(shop) = ui.shop {
        shop::render(buf, area, ctx, &shop, ui.shop_sel);
    }
    if let Some(codex) = ui.codex {
        codex::render(buf, area, ctx, &codex, &mut ui.codex_scroll);
    }
    // Les portails recouvrent tout l'écran.
    if let Some(portal) = ui.portal {
        portals::render(buf, area, ctx, &portal, &vis.stars);
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
