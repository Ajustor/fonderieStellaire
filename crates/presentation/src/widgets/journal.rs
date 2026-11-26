//! Journal des événements, écrit en machine à écrire.

use std::collections::VecDeque;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use crate::draw;
use crate::fmt;
use crate::fx::effects::typewriter;
use crate::fx::Rgb;
use crate::theme;

const CAPACITY: usize = 60;
/// Vitesse de la machine à écrire (caractères par seconde).
pub const TYPE_CPS: f64 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Achievement,
    Comet,
    Milestone,
    Save,
    Purchase,
    Supernova,
    Info,
    Error,
}

impl Kind {
    pub fn color(self) -> Rgb {
        match self {
            Kind::Achievement => theme::GOLD,
            Kind::Comet => theme::PINK,
            Kind::Milestone => theme::CYAN,
            Kind::Save => theme::GREY,
            Kind::Purchase => theme::GREEN,
            Kind::Supernova => theme::ORANGE,
            Kind::Info => theme::TEXT,
            Kind::Error => theme::RED,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub stamp: String,
    pub text: String,
    pub kind: Kind,
    /// Instant d'ajout (secondes depuis le lancement), pour la machine à écrire.
    pub born: f64,
}

#[derive(Debug, Default)]
pub struct Journal {
    entries: VecDeque<Entry>,
}

impl Journal {
    pub fn push(&mut self, play_time: f64, now: f64, kind: Kind, text: impl Into<String>) {
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        // Les entrées arrivées dans la même frame s'écrivent l'une après l'autre.
        let born = self
            .entries
            .back()
            .map(|e| {
                let done = e.born + e.text.chars().count() as f64 / TYPE_CPS;
                if done > now {
                    done.min(now + 1.5)
                } else {
                    now
                }
            })
            .unwrap_or(now);
        self.entries.push_back(Entry {
            stamp: fmt::clock(play_time),
            text: text.into(),
            kind,
            born,
        });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Entry> {
        self.entries.iter()
    }

    pub fn last(&self) -> Option<&Entry> {
        self.entries.back()
    }
}

/// `scroll` : nombre de lignes remontées (0 = les plus récentes), borné ici.
pub fn render(buf: &mut Buffer, area: Rect, journal: &Journal, now: f64, scroll: &mut usize) {
    let rows = area.height.saturating_sub(2) as usize;
    let max_scroll = journal.len().saturating_sub(rows);
    *scroll = (*scroll).min(max_scroll);
    let mut spans = vec![Span::styled(" Journal ", theme::title_style(false))];
    if *scroll > 0 {
        spans.push(Span::styled(
            format!(
                "↑ {} plus ancien{} · [PgDn] ",
                *scroll,
                if *scroll > 1 { "s" } else { "" }
            ),
            theme::fg(theme::GOLD),
        ));
    } else if max_scroll > 0 {
        spans.push(Span::styled("[PgUp] historique ", theme::fg(theme::GREY)));
    }
    let inner = draw::panel(buf, area, Line::from(spans), false);
    if inner.is_empty() {
        return;
    }
    let skip = journal.len().saturating_sub(rows + *scroll);
    for (row, e) in journal.iter().skip(skip).take(rows).enumerate() {
        let y = inner.y + row as u16;
        let x = draw::text(
            buf,
            inner.x + 1,
            y,
            &e.stamp,
            theme::fg(theme::GREY),
            inner.right(),
        );
        let shown = typewriter(&e.text, now - e.born, TYPE_CPS);
        let max = inner.right().saturating_sub(x + 2) as usize;
        let shown = draw::truncate(shown, max);
        let after = draw::text(
            buf,
            x + 1,
            y,
            &shown,
            theme::fg(e.kind.color()),
            inner.right(),
        );
        // Curseur clignotant pendant la frappe.
        if shown.chars().count() < e.text.chars().count().min(max) && after < inner.right() {
            draw::put(
                buf,
                inner,
                after as i32,
                y as i32,
                '▌',
                e.kind.color().dim(0.3),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_and_order() {
        let mut j = Journal::default();
        for i in 0..70 {
            j.push(i as f64, 0.0, Kind::Info, format!("m{i}"));
        }
        assert_eq!(j.len(), CAPACITY);
        assert_eq!(j.last().map(|e| e.text.as_str()), Some("m69"));
    }

    #[test]
    fn simultaneous_entries_are_sequenced() {
        let mut j = Journal::default();
        j.push(0.0, 10.0, Kind::Info, "a".repeat(60));
        j.push(0.0, 10.0, Kind::Info, "b");
        let born: Vec<f64> = j.iter().map(|e| e.born).collect();
        assert_eq!(born[0], 10.0);
        assert!((born[1] - 11.0).abs() < 1e-9);
    }
}
