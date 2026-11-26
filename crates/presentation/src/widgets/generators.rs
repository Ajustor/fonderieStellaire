//! Panneau des générateurs : 2 lignes par générateur, barre d'abordabilité,
//! ligne mystère, flash d'achat, shake d'échec, décryptage à la révélation.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use super::Ctx;
use application::GeneratorView;
use domain::generator::MILESTONES;

use crate::draw;
use crate::fmt;
use crate::fx::color::gradient;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::fx::rng::hash3;
use crate::theme;
use crate::ui::{keep_visible, Panel, Ui};
use crate::visuals::Visuals;

const DECRYPT_GLYPHS: [char; 6] = ['▓', '▒', '░', '█', '╳', '◆'];
pub const REVEAL_TIME: f64 = 0.9;
/// Largeur réservée au coût du lot (« ×25 999.9Qid »).
const COST_COL: u16 = 11;
/// Largeur réservée à la part de production (« (100%) »).
const PCT_COL: u16 = 6;

/// Texte « en cours de décryptage » : chaque caractère se fige de gauche à droite.
pub fn decrypt_char(
    real: char,
    index: usize,
    len: usize,
    progress: f64,
    now: f64,
    salt: i64,
) -> char {
    if real == ' ' {
        return ' ';
    }
    let reveal_at = index as f64 / len.max(1) as f64 * 0.8 + hash3(index as i64, salt, 1) * 0.2;
    if progress >= reveal_at {
        real
    } else {
        let k = hash3(index as i64, (now * 20.0) as i64, salt);
        DECRYPT_GLYPHS[(k * DECRYPT_GLYPHS.len() as f64) as usize % DECRYPT_GLYPHS.len()]
    }
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, vis: &mut Visuals, ui: &mut Ui) {
    let focused = ui.focus == Panel::Generators;
    let snap = ctx.snap;
    let title = Line::from(vec![
        Span::styled(" Générateurs ", theme::title_style(focused)),
        Span::styled("─ achat ", theme::fg(theme::BORDER)),
        Span::styled(
            format!("{} ", snap.buy_mode.label()),
            theme::bold(theme::GOLD),
        ),
    ]);
    let inner = draw::panel(buf, area, title, focused);
    ui.gen_rows = [None; 9];
    if inner.is_empty() {
        return;
    }
    let now = ctx.now;
    let count = snap.generators.len();
    if count == 0 {
        return;
    }
    ui.gen_sel = ui.gen_sel.min(count - 1);
    // Encart de détail en bas : séparateur + 2 lignes (ou 1 si peu de place).
    let detail_h = if inner.height >= 14 {
        3
    } else if inner.height >= 10 {
        2
    } else {
        0
    };
    let list = Rect::new(inner.x, inner.y, inner.width, inner.height - detail_h);
    let visible = (list.height / 2).max(1) as usize;
    keep_visible(ui.gen_sel, &mut ui.gen_scroll, visible);
    // Si tout tient, on ne défile pas (laisse de la place à la ligne mystère).
    if count <= visible {
        ui.gen_scroll = 0;
    }

    let mut y = inner.y;
    for (idx, g) in snap.generators.iter().enumerate().skip(ui.gen_scroll) {
        if y + 1 >= list.bottom() {
            break;
        }
        let gi = g.id.index();
        let Some(def) = ctx.catalog.generator(g.id) else {
            continue;
        };
        let color = theme::generator_color(gi);
        let selected = idx == ui.gen_sel;
        let row = Rect::new(inner.x, y, inner.width, 2);
        ui.gen_rows[gi.min(8)] = Some(y);

        if selected {
            draw::fill_bg(buf, row, theme::SELECT_BG);
        }

        // Ligne 1 : curseur, symbole, nom, possédés, prod/s.
        let cursor_color = if focused { theme::FOCUS } else { theme::MUTED };
        let max_x = inner.right();
        let mut x = inner.x;
        x = draw::text(
            buf,
            x,
            y,
            if selected { "▶ " } else { "  " },
            theme::bold(cursor_color),
            max_x,
        );
        x = draw::text(
            buf,
            x,
            y,
            &def.symbol.to_string(),
            theme::bold(color),
            max_x,
        );
        x = draw::text(buf, x, y, " ", theme::fg(theme::TEXT), max_x);

        let right = format!("×{:<4} {:>8}/s", g.owned, fmt::short(g.production));
        let right_w = right.chars().count() as u16;
        let name_max = max_x.saturating_sub(right_w + 1);
        let reveal = (now - vis.gen_reveal[gi.min(8)]) / REVEAL_TIME;
        let name_style = if selected {
            theme::bold(theme::TEXT)
        } else {
            theme::fg(theme::TEXT)
        };
        if (0.0..1.0).contains(&reveal) {
            let len = def.name.chars().count();
            for (i, ch) in def.name.chars().enumerate() {
                let cx = x + i as u16;
                if cx >= name_max {
                    break;
                }
                let shown = decrypt_char(ch, i, len, reveal, now, gi as i64);
                let c = if shown == ch {
                    theme::TEXT
                } else {
                    color.glow(0.3)
                };
                draw::put(buf, inner, cx as i32, y as i32, shown, c);
            }
        } else {
            draw::text(
                buf,
                x,
                y,
                &draw::truncate(def.name, name_max.saturating_sub(x) as usize),
                name_style,
                name_max,
            );
        }
        let rx = max_x.saturating_sub(right_w);
        draw::text(
            buf,
            rx,
            y,
            &format!("×{:<4}", g.owned),
            theme::bold(theme::GOLD),
            max_x,
        );
        draw::text(
            buf,
            rx + 6,
            y,
            &format!("{:>8}/s", fmt::short(g.production)),
            theme::fg(theme::MUTED),
            max_x,
        );

        // Ligne 2 : barre vers l'abordabilité, coût du lot, part de la production.
        let y2 = y + 1;
        let cost_txt = if g.quantity > 1 {
            format!("×{} {}", g.quantity, fmt::short(g.cost))
        } else {
            fmt::short(g.cost)
        };
        let pct = format!("({}%)", (g.share * 100.0).round() as i64);
        let fail = (now - vis.gen_fail[gi.min(8)]) / 0.3;
        let blinking = (0.0..1.0).contains(&fail) && ((now * 14.0) as i64) % 2 == 0;
        let cost_color = if blinking {
            theme::RED
        } else if (0.0..1.0).contains(&fail) {
            theme::RED.lerp(theme::GREY, fail)
        } else if g.affordable {
            theme::GREEN
        } else {
            theme::GREY
        };
        // Colonnes de largeur fixe : la barre garde la même taille quels que
        // soient les nombres affichés, et les coûts s'alignent d'une ligne à l'autre.
        let tail_w = COST_COL + PCT_COL + 3;
        let bar_x = inner.x + 4;
        let bar_w = inner.width.saturating_sub(4 + tail_w).max(4);
        let target = if g.cost > 0.0 {
            (snap.stardust / g.cost).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let fill = &mut vis.bar_fill[gi.min(8)];
        let k = 1.0 - (-ctx.dt * 10.0).exp();
        *fill += (target - *fill) * k;
        if (target - *fill).abs() < 0.002 {
            *fill = target;
        }
        let filled = (*fill * bar_w as f64).round() as u16;
        let shimmer_pos = ((now * 0.7 + gi as f64 * 0.37).fract() * 1.8 - 0.4) * bar_w as f64;
        for i in 0..bar_w {
            let cx = bar_x + i;
            if cx >= inner.right() {
                break;
            }
            if i < filled {
                let mut c = if g.affordable {
                    theme::GREEN
                } else {
                    gradient(&[color.dim(0.7), color], i as f64 / bar_w.max(1) as f64)
                };
                if g.affordable && !ctx.reduced {
                    let d = (i as f64 - shimmer_pos).abs();
                    if d < 2.5 {
                        c = c.glow(0.55 * (1.0 - d / 2.5));
                    }
                }
                draw::put(buf, inner, cx as i32, y2 as i32, '█', c);
            } else {
                draw::put(buf, inner, cx as i32, y2 as i32, '░', theme::BAR_EMPTY);
            }
        }
        let cost_end = bar_x + bar_w + 2 + COST_COL;
        let cost_txt = draw::truncate(&cost_txt, COST_COL as usize);
        draw::text_right(
            buf,
            cost_end,
            y2,
            &cost_txt,
            theme::fg(cost_color).add_modifier(Modifier::BOLD),
            bar_x + bar_w + 1,
        );
        draw::text_right(
            buf,
            cost_end + 1 + PCT_COL,
            y2,
            &pct,
            theme::fg(theme::MUTED),
            cost_end + 1,
        );

        // Flash d'achat : le fond s'estompe de la couleur du générateur vers transparent.
        let flash = (now - vis.gen_flash[gi.min(8)]) / 0.4;
        if (0.0..1.0).contains(&flash) {
            let base = if selected {
                theme::SELECT_BG
            } else {
                theme::BLACK
            };
            let amount = 1.0 - ease_out_cubic(flash);
            let bg = base.lerp(color.dim(0.35), amount);
            draw::fill_bg(buf, row, bg);
        }
        // Shake d'échec : 2 oscillations horizontales.
        if (0.0..1.0).contains(&fail) && !ctx.reduced {
            let dx =
                ((fail * std::f64::consts::TAU * 2.0).sin() * 2.0 * (1.0 - fail)).round() as i32;
            effects::offset(buf, row, dx, 0);
        }
        y += 2;
    }

    // Ligne mystère (teaser).
    if let Some(t) = snap.teaser {
        if y < list.bottom() && ui.gen_scroll + visible >= count {
            let txt = format!(
                "   ??? ─ encore {} ✦ à récolter",
                fmt::short(t.remaining.max(1.0))
            );
            let pulse = 0.5 + 0.5 * (now * 1.5).sin();
            let c = theme::GREY.lerp(theme::MUTED, pulse * 0.5);
            draw::text(buf, inner.x, y, &txt, theme::fg(c), inner.right());
        }
    }

    if detail_h > 0 {
        if let Some(g) = snap.generators.get(ui.gen_sel) {
            render_detail(buf, inner, detail_h, ctx, g);
        }
    }
}

/// Encart « qui fait quoi » du générateur sélectionné.
fn render_detail(buf: &mut Buffer, inner: Rect, detail_h: u16, ctx: &Ctx, g: &GeneratorView) {
    let Some(def) = ctx.catalog.generator(g.id) else {
        return;
    };
    let color = theme::generator_color(g.id.index());
    let sep_y = inner.bottom() - detail_h;
    let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
    draw::text(
        buf,
        inner.x + 1,
        sep_y,
        &sep,
        theme::fg(theme::BORDER),
        inner.right(),
    );
    let max = inner.width.saturating_sub(2) as usize;
    let mut y = sep_y + 1;

    if detail_h >= 3 {
        let x = draw::text(
            buf,
            inner.x + 1,
            y,
            &format!("{} ", def.symbol),
            theme::bold(color),
            inner.right(),
        );
        let text = draw::truncate(def.description, max.saturating_sub(2));
        draw::text(buf, x, y, &text, theme::fg(theme::MUTED), inner.right());
        y += 1;
    }

    // Production par unité (bonus compris), part du total, prochain palier.
    let per_unit = if g.owned > 0 {
        g.production / g.owned as f64
    } else {
        def.base_prod
    };
    let milestone = match MILESTONES.iter().find(|&&m| m > g.owned) {
        Some(&m) => format!("×2 à {m} (encore {})", m - g.owned),
        None => "tous les paliers atteints".to_string(),
    };
    let stats = format!(
        "{}/s par unité · {}% du total · {}",
        fmt::short(per_unit),
        (g.share * 100.0).round() as i64,
        milestone
    );
    let stats = draw::truncate(&stats, max);
    draw::text(
        buf,
        inner.x + 1,
        y,
        &stats,
        theme::fg(theme::CYAN),
        inner.right(),
    );
}
