//! Graphique de production : les 120 dernières secondes de prod/s, avec
//! valeur actuelle et tendance en titre, axe des valeurs à gauche et axe du
//! temps sur la bordure basse. Dessiné à la main (plusieurs lignes, dégradé).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::color::gradient;
use crate::fx::Rgb;
use crate::theme;

const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
/// Largeur maximale du graphique (colonnes), pour rester sans allocation.
const MAX_COLS: usize = 256;
/// Fenêtre de comparaison pour la tendance.
pub const TREND_WINDOW: usize = 30;

/// Normalise des `f64` (qui peuvent dépasser `u64`) en `u64` sur 0..=1000.
pub fn normalize(values: &[f64], out: &mut Vec<u64>) {
    out.clear();
    let max = values.iter().copied().fold(0.0f64, f64::max);
    if max <= 0.0 || !max.is_finite() {
        out.extend(values.iter().map(|_| 0));
        return;
    }
    out.extend(
        values
            .iter()
            .map(|v| ((v / max).clamp(0.0, 1.0) * 1000.0).round() as u64),
    );
}

/// Bornes de l'axe vertical : du min au max de la fenêtre, pour que les
/// variations se voient (la production ne fait que croître dans un idle).
/// Une courbe plate est centrée.
pub fn axis_range(values: &[f64]) -> (f64, f64) {
    let hi = values.iter().copied().fold(0.0f64, f64::max);
    let lo = values
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min)
        .max(0.0);
    if !hi.is_finite() || hi <= 0.0 {
        return (0.0, 1.0);
    }
    if hi - lo < hi * 0.02 {
        // Quasi constant : bande de ±5 % autour de la valeur.
        return ((hi * 0.95).max(0.0), hi * 1.05);
    }
    // Petite marge sous le minimum pour que la plus basse colonne reste visible.
    ((lo - (hi - lo) * 0.1).max(0.0), hi)
}

/// Normalise sur 0..=1000 entre `lo` et `hi` (au moins 1 pour une valeur > 0).
pub fn normalize_range(values: &[f64], lo: f64, hi: f64, out: &mut Vec<u64>) {
    out.clear();
    let span = (hi - lo).max(f64::MIN_POSITIVE);
    out.extend(values.iter().map(|&v| {
        let n = (((v - lo) / span).clamp(0.0, 1.0) * 1000.0).round() as u64;
        if v > 0.0 {
            n.max(1)
        } else {
            n
        }
    }));
}

/// Ramène `values` à au plus `cols` colonnes (moyenne par paquet).
/// Renvoie le nombre de colonnes écrites dans `out`.
pub fn bucket(values: &[f64], cols: usize, out: &mut [f64]) -> usize {
    let cols = cols.min(out.len());
    if values.len() <= cols {
        out[..values.len()].copy_from_slice(values);
        return values.len();
    }
    let n = values.len();
    for (i, slot) in out.iter_mut().enumerate().take(cols) {
        let a = i * n / cols;
        let b = ((i + 1) * n / cols).max(a + 1);
        *slot = values[a..b].iter().sum::<f64>() / (b - a) as f64;
    }
    cols
}

/// Nombre maximal de générateurs distingués par couleur.
pub const MAX_GENERATORS: usize = 16;

/// Moyenne, par colonne, de la production de chaque générateur (mêmes paquets
/// que [`bucket`]). `mix` est à plat : `len` échantillons de `width` valeurs.
/// Renvoie le nombre de générateurs pris en compte.
pub fn bucket_mix(
    mix: &[f64],
    width: usize,
    len: usize,
    cols: usize,
    out: &mut [[f64; MAX_GENERATORS]],
) -> usize {
    let gens = width.min(MAX_GENERATORS);
    if width == 0 || mix.len() < len * width {
        for row in out.iter_mut().take(cols) {
            *row = [0.0; MAX_GENERATORS];
        }
        return 0;
    }
    let cols = cols.min(out.len()).min(len);
    for (i, row) in out.iter_mut().enumerate().take(cols) {
        let (a, b) = if len <= cols {
            (i, i + 1)
        } else {
            let a = i * len / cols;
            (a, ((i + 1) * len / cols).max(a + 1))
        };
        *row = [0.0; MAX_GENERATORS];
        for s in a..b {
            for (g, slot) in row.iter_mut().enumerate().take(gens) {
                *slot += mix[s * width + g];
            }
        }
        let k = (b - a) as f64;
        row.iter_mut().for_each(|v| *v /= k);
    }
    gens
}

/// Générateur qui occupe la hauteur `probe` d'une colonne de hauteur `height`,
/// les systèmes étant empilés du premier (en bas) au dernier (en haut) au
/// prorata de leur production.
pub fn system_at(shares: &[f64], height: f64, probe: f64) -> Option<usize> {
    let total: f64 = shares.iter().sum();
    if total <= 0.0 || height <= 0.0 {
        return None;
    }
    let mut top = 0.0;
    let mut last = None;
    for (g, &v) in shares.iter().enumerate() {
        if v <= 0.0 {
            continue;
        }
        top += height * v / total;
        last = Some(g);
        if probe < top {
            return Some(g);
        }
    }
    last
}

/// Légende : principaux producteurs actuels (symbole coloré + part), écrite
/// alignée à droite sur la bordure haute, entre `min_x` et `right`.
fn draw_legend(buf: &mut Buffer, ctx: &Ctx, y: u16, min_x: u16, right: u16) {
    let mut top = [(0usize, 0.0f64); MAX_GENERATORS];
    let mut n = 0;
    for g in ctx.snap.generators.iter().filter(|g| g.share >= 0.01) {
        if n < MAX_GENERATORS {
            top[n] = (g.id.index(), g.share);
            n += 1;
        }
    }
    let top = &mut top[..n];
    top.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    // Garde les plus grosses parts tant qu'elles tiennent, puis les écrit de
    // gauche à droite (la plus grosse en premier), alignées à droite.
    let entry_w = |share: f64| fmt::percent(share).chars().count() as u16 + 1; // symbole + %
    let room = right.saturating_sub(min_x);
    let mut total = 0u16;
    let mut count = 0;
    for &(_, share) in top.iter() {
        let w = entry_w(share) + 2; // séparateurs « ─ »
        if total + w > room {
            break;
        }
        total += w;
        count += 1;
    }
    let mut shown = [(0usize, 0.0f64, 0u16); MAX_GENERATORS];
    let mut x = right.saturating_sub(total) + 1;
    for (slot, &(g, share)) in shown.iter_mut().zip(top.iter()).take(count) {
        *slot = (g, share, x);
        x += entry_w(share) + 2;
    }
    for &(g, share, sx) in &shown[..count] {
        let symbol = ctx
            .catalog
            .generators
            .get(g)
            .map(|d| d.symbol)
            .unwrap_or('?');
        let color = theme::generator_color(g);
        draw::put_styled(
            buf,
            buf.area,
            sx as i32,
            y as i32,
            symbol,
            theme::bold(color),
        );
        draw::text(
            buf,
            sx + 1,
            y,
            &fmt::percent(share),
            theme::fg(color),
            right,
        );
    }
}

/// Variation relative entre la valeur d'il y a `TREND_WINDOW` s et la dernière.
pub fn trend(history: &[f64]) -> Option<f64> {
    let last = *history.last()?;
    if history.len() < 2 {
        return None;
    }
    let past = history[history.len().saturating_sub(TREND_WINDOW + 1)];
    if past <= 0.0 {
        return if last > 0.0 {
            Some(f64::INFINITY)
        } else {
            None
        };
    }
    Some(last / past - 1.0)
}

fn trend_label(t: Option<f64>) -> (String, Rgb) {
    match t {
        Some(v) if v.is_infinite() => ("▲ nouveau".into(), theme::GREEN),
        Some(v) if v > 0.01 => (format!("▲ +{}", fmt::percent(v)), theme::GREEN),
        Some(v) if v < -0.01 => (format!("▼ {}", fmt::percent(v)), theme::RED),
        Some(_) => ("= stable".into(), theme::GREY),
        None => (String::new(), theme::GREY),
    }
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, scratch: &mut Vec<u64>) {
    let snap = ctx.snap;
    let history = &snap.history[..];
    let current = history.last().copied().unwrap_or(snap.production);
    let (trend_txt, trend_color) = trend_label(trend(history));

    let mut title = vec![
        Span::styled(" Production ", theme::title_style(false)),
        Span::styled(
            format!("{}/s ", fmt::short(current)),
            theme::bold(theme::GOLD),
        ),
    ];
    if !trend_txt.is_empty() {
        title.push(Span::styled(
            format!("{trend_txt} "),
            theme::bold(trend_color),
        ));
    }
    let title_w: usize = title.iter().map(|s| s.content.chars().count()).sum();
    let inner = draw::panel(buf, area, Line::from(title), false);
    if inner.height == 0 || inner.width < 12 {
        return;
    }
    draw_legend(
        buf,
        ctx,
        area.y,
        area.x + 1 + title_w as u16 + 1,
        area.right().saturating_sub(2),
    );
    if history.is_empty() {
        draw::text(
            buf,
            inner.x + 1,
            inner.y,
            "mesure en cours… (1 point par seconde)",
            theme::fg(theme::GREY),
            inner.right(),
        );
        return;
    }

    // --- Axe des valeurs ---
    let (lo, hi) = axis_range(history);
    let rows = inner.height as usize;
    let top_label = fmt::short(hi);
    let mid_label = fmt::short((lo + hi) / 2.0);
    let bottom_label = fmt::short(lo);
    let label_w = [&top_label, &mid_label, &bottom_label]
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(1) as u16;
    let gutter = label_w + 1;
    let axis = theme::BORDER;
    for r in 0..rows {
        let y = inner.y + r as u16;
        let label: Option<&str> = if r == 0 {
            Some(&top_label)
        } else if r == rows - 1 {
            Some(&bottom_label)
        } else if rows >= 4 && r == rows / 2 {
            Some(&mid_label)
        } else {
            None
        };
        if let Some(l) = label {
            draw::text_right(
                buf,
                inner.x + label_w,
                y,
                l,
                theme::fg(theme::MUTED),
                inner.x,
            );
            draw::put(buf, inner, (inner.x + label_w) as i32, y as i32, '┤', axis);
        } else {
            draw::put(buf, inner, (inner.x + label_w) as i32, y as i32, '│', axis);
        }
    }

    // --- Colonnes (les 120 s tiennent toujours dans la largeur) ---
    let chart_x = inner.x + gutter;
    let chart_w = inner.width.saturating_sub(gutter) as usize;
    let mut cols = [0.0f64; MAX_COLS];
    let n = bucket(history, chart_w, &mut cols);
    normalize_range(&cols[..n], lo, hi, scratch);
    let x0 = chart_x + (chart_w - n) as u16;
    let mut mix = [[0.0f64; MAX_GENERATORS]; MAX_COLS];
    let gens = bucket_mix(
        &snap.history_mix,
        snap.history_mix_width,
        history.len(),
        n,
        &mut mix,
    );

    // Ligne de base sur toute la largeur (la partie vide = pas encore mesuré).
    for i in 0..chart_w {
        let x = chart_x + i as u16;
        let c = if (x as usize) < x0 as usize {
            theme::BAR_EMPTY.dim(0.4)
        } else {
            theme::BAR_EMPTY
        };
        draw::put(buf, inner, x as i32, (inner.bottom() - 1) as i32, '·', c);
    }
    for (i, &v) in scratch.iter().enumerate() {
        let eighths = v as usize * rows * 8 / 1000;
        let x = x0 + i as u16;
        let is_now = i + 1 == n;
        for r in 0..rows {
            let level = eighths.saturating_sub(r * 8);
            if level == 0 {
                continue;
            }
            let y = inner.bottom() - 1 - r as u16;
            let ch = LEVELS[level.min(8) - 1];
            // Couleur du système qui produit cette tranche de la colonne.
            let filled_top = eighths.min(r * 8 + 8);
            let probe = (r * 8 + filled_top) as f64 / 2.0;
            let mut c = match system_at(&mix[i][..gens], eighths as f64, probe) {
                Some(g) => theme::generator_color(g),
                None => gradient(
                    &[theme::CYAN.dim(0.3), theme::CYAN, theme::GOLD],
                    (r as f64 + 1.0) / rows as f64,
                ),
            };
            if is_now {
                c = c.glow(0.5 + 0.2 * (ctx.now * 4.0).sin());
            }
            draw::put(buf, inner, x as i32, y as i32, ch, c);
        }
    }

    // --- Axe du temps, sur la bordure basse ---
    let by = area.bottom() - 1;
    let now_label = " maintenant ";
    let now_x = area
        .right()
        .saturating_sub(now_label.chars().count() as u16 + 1);
    let span = format!(" −{} ", fmt::duration(history.len() as f64));
    let span_x = x0.saturating_sub(1).max(area.x + 1);
    if span_x + span.chars().count() as u16 + 1 < now_x {
        draw::text(buf, span_x, by, &span, theme::fg(theme::MUTED), now_x);
    }
    draw::text(
        buf,
        now_x,
        by,
        now_label,
        theme::fg(theme::MUTED),
        area.right() - 1,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_handles_huge_values() {
        let mut out = Vec::new();
        normalize(&[0.0, 5e30, 1e31], &mut out);
        assert_eq!(out, [0, 500, 1000]);
        normalize(&[0.0, 0.0], &mut out);
        assert_eq!(out, [0, 0]);
    }

    #[test]
    fn axis_follows_the_window() {
        let (lo, hi) = axis_range(&[2800.0, 3000.0, 3500.0]);
        assert_eq!(hi, 3500.0);
        assert!(lo > 2600.0 && lo < 2800.0, "{lo}");
        let (lo, hi) = axis_range(&[100.0, 100.0]);
        assert!(lo < 100.0 && hi > 100.0, "courbe plate centrée");
        assert_eq!(axis_range(&[0.0, 0.0]), (0.0, 1.0));
        let mut out = Vec::new();
        normalize_range(&[2800.0, 3500.0], 2700.0, 3500.0, &mut out);
        assert!(out[0] >= 1 && out[0] < 300);
        assert_eq!(out[1], 1000);
    }

    #[test]
    fn stacked_systems() {
        // 25 % sonde (en bas), 75 % drone (au-dessus), colonne de 16 huitièmes.
        let shares = [1.0, 3.0];
        assert_eq!(system_at(&shares, 16.0, 2.0), Some(0));
        assert_eq!(system_at(&shares, 16.0, 5.0), Some(1));
        assert_eq!(system_at(&shares, 16.0, 99.0), Some(1));
        assert_eq!(system_at(&[0.0, 0.0], 16.0, 1.0), None);
        assert_eq!(
            system_at(&[0.0, 2.0], 16.0, 1.0),
            Some(1),
            "saute les absents"
        );
    }

    #[test]
    fn mix_bucketing_matches_totals() {
        // 4 échantillons × 2 générateurs, ramenés à 2 colonnes.
        let mix = [1.0, 0.0, 3.0, 0.0, 0.0, 2.0, 0.0, 4.0];
        let mut out = [[0.0; MAX_GENERATORS]; 4];
        assert_eq!(bucket_mix(&mix, 2, 4, 2, &mut out), 2);
        assert_eq!(&out[0][..2], &[2.0, 0.0]);
        assert_eq!(&out[1][..2], &[0.0, 3.0]);
        // Données incohérentes : pas de couleurs, pas de panique.
        assert_eq!(bucket_mix(&mix[..3], 2, 4, 2, &mut out), 0);
    }

    #[test]
    fn bucketing_keeps_all_history() {
        let values: Vec<f64> = (0..120).map(|i| i as f64).collect();
        let mut out = [0.0; MAX_COLS];
        assert_eq!(bucket(&values, 40, &mut out), 40);
        assert_eq!(out[0], 1.0); // moyenne de 0,1,2
        assert_eq!(out[39], 118.0); // moyenne de 117,118,119
        assert_eq!(bucket(&values[..10], 40, &mut out), 10);
        assert_eq!(out[9], 9.0);
    }

    #[test]
    fn trends() {
        assert_eq!(trend(&[]), None);
        assert_eq!(trend(&[5.0]), None);
        let mut h = vec![100.0; 40];
        assert_eq!(trend(&h), Some(0.0));
        *h.last_mut().unwrap() = 150.0;
        assert!((trend(&h).unwrap() - 0.5).abs() < 1e-12);
        assert_eq!(trend(&[0.0, 3.0]), Some(f64::INFINITY));
        assert_eq!(trend_label(Some(-0.2)).0, "▼ -20%");
    }
}
