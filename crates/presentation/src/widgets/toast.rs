//! Toasts : glissent depuis la droite (ease_out_back), restent 3 s, repartent.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use crate::draw;
use crate::fx::easing::{ease_in_cubic, ease_out_back};
use crate::fx::effects::typewriter;
use crate::fx::Rgb;
use crate::theme;

pub const SLIDE_IN: f64 = 0.45;
pub const STAY: f64 = 3.0;
pub const SLIDE_OUT: f64 = 0.35;
const MAX_TOASTS: usize = 3;
/// Hauteur d'un toast dont le corps tient sur une ligne.
const HEIGHT: u16 = 4;

#[derive(Clone, Debug)]
pub struct Toast {
    pub title: String,
    pub body: String,
    pub color: Rgb,
    pub born: f64,
}

impl Toast {
    pub fn lifetime() -> f64 {
        SLIDE_IN + STAY + SLIDE_OUT
    }

    /// Fraction de la largeur encore cachée hors écran (0 = en place).
    pub fn hidden(&self, now: f64) -> f64 {
        let age = now - self.born;
        if age < SLIDE_IN {
            1.0 - ease_out_back(age / SLIDE_IN)
        } else if age < SLIDE_IN + STAY {
            0.0
        } else {
            ease_in_cubic((age - SLIDE_IN - STAY) / SLIDE_OUT)
        }
    }
}

#[derive(Debug, Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn push(
        &mut self,
        now: f64,
        color: Rgb,
        title: impl Into<String>,
        body: impl Into<String>,
    ) {
        if self.items.len() >= MAX_TOASTS {
            self.items.remove(0);
        }
        self.items.push(Toast {
            title: title.into(),
            body: body.into(),
            color,
            born: now,
        });
    }

    pub fn update(&mut self, now: f64) {
        self.items.retain(|t| now - t.born < Toast::lifetime());
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Dessine les toasts empilés en haut à droite de `area`, à partir de `top`.
    pub fn render(&self, buf: &mut Buffer, area: Rect, top: u16, now: f64) {
        // Largeur ajustée au texte (le corps peut passer sur deux lignes).
        let max_w = (area.width * 3 / 5)
            .max(44)
            .min(area.width.saturating_sub(2));
        let mut y = top;
        for t in self.items.iter() {
            let title_w = t.title.chars().count() as u16 + 4;
            let body_w = t.body.chars().count() as u16 + 4;
            let width = title_w.max(body_w.min(max_w)).clamp(24, max_w.max(24));
            let body_lines = draw::wrap(&t.body, width.saturating_sub(3) as usize);
            let height = HEIGHT - 1 + body_lines.len().clamp(1, 3) as u16;
            if y + height > area.bottom() {
                break;
            }
            let row_y = y;
            y += height;
            let y = row_y;
            let hidden = t.hidden(now);
            let final_x = area.right() as f64 - width as f64 - 1.0;
            let x = final_x + hidden * (width as f64 + 2.0);
            let x = x.round() as i32;
            if x >= area.right() as i32 {
                continue;
            }
            let full = Rect::new(x.max(0) as u16, y, width, height);
            let rect = area.intersection(full);
            if rect.width < 3 {
                continue;
            }
            draw::clear(buf, rect);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme::fg(t.color));
            let inner = block.inner(rect);
            block.render(rect, buf);
            let age = now - t.born - SLIDE_IN * 0.5;
            let max = inner.width.saturating_sub(1) as usize;
            // Pendant la glissade, le bord de l'écran coupe le texte : normal.
            let cut = |s: &str| {
                if hidden > 0.0 {
                    draw::fit(s, max)
                } else {
                    draw::truncate(s, max)
                }
            };
            let title = typewriter(&t.title, age, 45.0);
            draw::text(
                buf,
                inner.x + 1,
                inner.y,
                &cut(title),
                theme::bold(t.color),
                inner.right(),
            );
            let mut typed = age - t.title.chars().count() as f64 / 45.0;
            for (k, line) in body_lines.iter().enumerate() {
                if inner.height as usize <= k + 1 {
                    break;
                }
                let shown = typewriter(line, typed, 50.0);
                typed -= line.chars().count() as f64 / 50.0;
                draw::text(
                    buf,
                    inner.x + 1,
                    inner.y + 1 + k as u16,
                    &cut(shown),
                    theme::fg(theme::TEXT),
                    inner.right(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slides_in_stays_and_leaves() {
        let t = Toast {
            title: "a".into(),
            body: "b".into(),
            color: theme::GOLD,
            born: 0.0,
        };
        assert!((t.hidden(0.0) - 1.0).abs() < 1e-9);
        assert_eq!(t.hidden(1.0), 0.0);
        assert!(t.hidden(SLIDE_IN + STAY + SLIDE_OUT * 0.5) > 0.0);
        let mut ts = Toasts::default();
        ts.push(0.0, theme::GOLD, "x", "y");
        ts.update(10.0);
        assert!(ts.is_empty());
    }
}
