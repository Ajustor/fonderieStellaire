//! Synchronisation entre appareils (Options › Synchronisation) : QR code et
//! code du lien, état de la dernière synchronisation.

use qrcode::{Color as QrColor, EcLevel, QrCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use super::Ctx;
use crate::draw;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::fx::Rgb;
use crate::layout::centered;
use crate::theme;
use crate::ui::Popup;

/// Ce que l'écran montre de la synchronisation (préparé par `App`).
#[derive(Clone, Debug, PartialEq)]
pub struct SyncView {
    /// Un relais est configuré dans cette version.
    pub available: bool,
    /// Code du lien (`XXXXX-XXXXX-…`), si l'appareil est lié.
    pub code: Option<String>,
    /// Une demande est en cours.
    pub busy: bool,
    /// État en clair (« Synchronisé il y a 12 s »…).
    pub status: String,
    pub error: bool,
    /// Pseudo du classement (facultatif).
    pub pseudo: Option<String>,
    /// Saisie du pseudo en cours (texte tapé).
    pub pseudo_input: Option<String>,
}

impl SyncView {
    /// Ligne du pseudo (appareil lié seulement).
    pub fn pseudo_line(&self) -> Option<(String, Rgb)> {
        self.code.as_ref()?;
        Some(match (&self.pseudo_input, &self.pseudo) {
            (Some(t), _) => (format!("Pseudo : {t}▌"), theme::GOLD),
            (None, Some(p)) => (format!("★ Classement : {p}"), theme::GOLD),
            (None, None) => (
                "Classement : aucun pseudo (facultatif)".to_string(),
                theme::MUTED,
            ),
        })
    }
}

impl SyncView {
    /// Valeur courte pour la ligne des Options.
    pub fn short(&self) -> &'static str {
        match (self.available, &self.code, self.error) {
            (false, _, _) => "indisponible",
            (true, None, _) => "non liée",
            (true, Some(_), true) => "liée · hors ligne",
            (true, Some(_), false) => "liée",
        }
    }
}

/// Contenu du QR code : préfixe et code, en caractères du mode alphanumérique.
pub fn qr_payload(code: &str) -> String {
    format!("FONDERIE:{code}")
}

/// Modules du QR code (vrai = sombre), marge de 2 modules comprise.
pub fn qr_modules(text: &str) -> Option<(usize, Vec<bool>)> {
    let qr = QrCode::with_error_correction_level(text.as_bytes(), EcLevel::L).ok()?;
    let n = qr.width();
    let quiet = 2;
    let size = n + 2 * quiet;
    let colors = qr.to_colors();
    let mut out = vec![false; size * size];
    for y in 0..n {
        for x in 0..n {
            out[(y + quiet) * size + x + quiet] = colors[y * n + x] == QrColor::Dark;
        }
    }
    Some((size, out))
}

/// Dessine le QR code en demi-blocs (deux modules par cellule), clair sur sombre
/// inversé : les lecteurs veulent des modules sombres sur fond clair.
fn draw_qr(buf: &mut Buffer, x: u16, y: u16, size: usize, modules: &[bool]) {
    let color = |dark: bool| if dark { theme::BLACK } else { theme::WHITE };
    for row in 0..size.div_ceil(2) {
        for col in 0..size {
            let top = modules[(2 * row) * size + col];
            let bottom = modules
                .get((2 * row + 1) * size + col)
                .copied()
                .unwrap_or(false);
            if let Some(cell) = buf.cell_mut((x + col as u16, y + row as u16)) {
                cell.set_char('▀')
                    .set_fg(color(top).color())
                    .set_bg(color(bottom).color());
            }
        }
    }
}

fn centered_text(buf: &mut Buffer, inner: Rect, y: u16, s: &str, color: Rgb, bold: bool) {
    let w = s.chars().count() as u16;
    let x = inner.x + inner.width.saturating_sub(w) / 2;
    let style = if bold {
        theme::bold(color)
    } else {
        theme::fg(color)
    };
    draw::text(buf, x, y, s, style, inner.right());
}

pub fn render(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    popup: &Popup,
    armed: bool,
    view: &SyncView,
) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    effects::fade(buf, area, 0.6 * s);
    let width = area.width.saturating_sub(4).min(76);
    let text_w = width.saturating_sub(4) as usize;
    let max_h = area.height.saturating_sub(2);

    let intro = match &view.code {
        Some(code) => format!(
            "Sur le téléphone : Sauver › Synchroniser › Scanner. Sur un autre ordinateur : fonderie-stellaire --link {code}"
        ),
        None => "Joue sur plusieurs appareils avec la même partie, sans compte : un code de lien (QR code) relie cet ordinateur à ton téléphone. La partie la plus jouée est gardée, l'autre est mise de côté en .bak.".to_string(),
    };
    let mut intro = draw::wrap(&intro, text_w);
    let qr = view
        .code
        .as_deref()
        .and_then(|c| qr_modules(&qr_payload(c)));
    let footer = match (&view.code, armed) {
        _ if view.pseudo_input.is_some() => "[Entrée] valider le pseudo   [Échap] annuler",
        (Some(_), true) => "[Entrée] confirmer : délier cet appareil   [Échap] annuler",
        (Some(_), false) => "[Entrée] délier   [N] pseudo   [Échap] retour",
        (None, _) if view.busy => "[Échap] retour",
        (None, _) => "[Entrée] créer un lien   [Échap] retour",
    };
    let footer = if footer.chars().count() <= text_w {
        footer
    } else {
        "[Entrée] [Échap]"
    };
    let status = draw::wrap(&view.status, text_w);

    // Hauteur : bordures, intro + blanc, QR + blanc, code + blanc, état,
    // blanc, pied. Petits terminaux : on retire d'abord l'intro, puis les
    // lignes vides, et le QR code en dernier recours.
    let qr_rows = match &qr {
        Some((n, _)) if *n <= text_w => n.div_ceil(2) as u16,
        _ => 0,
    };
    let has_code = u16::from(view.code.is_some());
    let pseudo = view.pseudo_line();
    let status_h = status.len() as u16;
    let need = |intro_h: u16, qr_rows: u16, gap: u16| {
        let intro_h = if intro_h > 0 { intro_h + gap } else { 0 };
        let qr_h = if qr_rows > 0 { qr_rows + gap } else { 0 };
        2 + intro_h + qr_h + has_code * (2 + gap) + status_h + gap + 1
    };
    let mut qr_rows = qr_rows;
    let mut gap = 1;
    if need(intro.len() as u16, qr_rows, gap) > max_h {
        intro.clear();
    }
    if need(0, qr_rows, gap) > max_h {
        gap = 0;
    }
    if need(0, qr_rows, gap) > max_h {
        qr_rows = 0;
    }
    let height = need(intro.len() as u16, qr_rows, gap).min(max_h);
    let w = ((width as f64 * s).round() as u16).max(2);
    let h = ((height as f64 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let border = if armed { theme::RED } else { theme::CYAN };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(border))
        .title(
            Line::from(Span::styled(
                " ↕ SYNCHRONISATION ",
                theme::bold(theme::GOLD),
            ))
            .centered(),
        );
    let inner = block.inner(rect);
    block.render(rect, buf);
    if s < 0.95 || inner.height < 3 {
        return;
    }
    let right = inner.right();
    let mut y = inner.y;
    for l in &intro {
        draw::text(buf, inner.x + 1, y, l, theme::fg(theme::MUTED), right);
        y += 1;
    }
    if !intro.is_empty() {
        y += gap;
    }
    if let (Some((n, modules)), true) = (&qr, qr_rows > 0) {
        let x = inner.x + inner.width.saturating_sub(*n as u16) / 2;
        draw_qr(buf, x, y, *n, modules);
        y += qr_rows + gap;
    }
    if let Some(code) = &view.code {
        centered_text(buf, inner, y, code, theme::GOLD, true);
        y += 1;
    }
    if let Some((line, color)) = &pseudo {
        centered_text(buf, inner, y, line, *color, view.pseudo_input.is_some());
        y += 1 + gap;
    }
    let status_color = if view.error {
        theme::ORANGE
    } else if view.busy {
        theme::CYAN
    } else {
        theme::GREEN
    };
    for l in &status {
        centered_text(buf, inner, y, l, status_color, false);
        y += 1;
    }
    let fw = footer.chars().count() as u16;
    draw::text(
        buf,
        inner.x + inner.width.saturating_sub(fw) / 2,
        inner.bottom() - 1,
        footer,
        theme::fg(if armed { theme::ORANGE } else { theme::GREY }),
        right,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_code_fits_a_small_qr_code() {
        let code = "ABCDE-FGHJK-MNPQR-STVWX-YZ012";
        let (size, modules) = qr_modules(&qr_payload(code)).unwrap();
        // Version 2 (25 modules) + marges : 15 lignes de terminal.
        assert!(size <= 29, "QR de {size} modules");
        assert_eq!(modules.len(), size * size);
        assert!(modules.iter().any(|&m| m));
    }
}
