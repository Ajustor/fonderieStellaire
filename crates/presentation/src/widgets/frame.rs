//! Où un écran de la barre pose son cadre : en fenêtre centrée par-dessus la
//! fonderie (Android, écrans hérités du terminal), ou en onglet qui occupe
//! toute la zone de contenu (ordinateur).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Clear, Widget};

use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::layout::centered;
use crate::ui::Popup;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Frame {
    /// Fenêtre centrée qui grandit, fond assombri.
    #[default]
    Window,
    /// Onglet : toute la zone reçue, sans fondu ni agrandissement.
    Tab,
}

impl Frame {
    /// Largeur du cadre : bornée à `max` en fenêtre, toute la zone en onglet.
    pub fn width(self, area: Rect, max: u16) -> u16 {
        match self {
            Frame::Window => area.width.saturating_sub(4).min(max),
            Frame::Tab => area.width,
        }
    }

    /// Hauteur du cadre : celle voulue (bornée à l'écran) en fenêtre, toute la
    /// zone en onglet.
    pub fn height(self, area: Rect, wanted: u16) -> u16 {
        match self {
            Frame::Window => wanted.min(area.height.saturating_sub(2)),
            Frame::Tab => area.height,
        }
    }

    /// Pose le cadre et renvoie son rectangle, et si le contenu peut être
    /// dessiné (la fenêtre est presque ouverte). `None` : rien à dessiner.
    pub fn place(
        self,
        buf: &mut Buffer,
        area: Rect,
        now: f64,
        popup: &Popup,
        width: u16,
        height: u16,
    ) -> Option<(Rect, bool)> {
        self.place_faded(buf, area, now, popup, (width, height), 0.6)
    }

    /// Comme [`Frame::place`], avec l'assombrissement du fond propre à la
    /// fenêtre (`fade` × l'ouverture ; sans effet en onglet).
    pub fn place_faded(
        self,
        buf: &mut Buffer,
        area: Rect,
        now: f64,
        popup: &Popup,
        (width, height): (u16, u16),
        fade: f64,
    ) -> Option<(Rect, bool)> {
        match self {
            Frame::Window => {
                let s = ease_out_cubic(popup.scale(now));
                if s <= 0.0 {
                    return None;
                }
                effects::fade(buf, area, fade * s);
                let w = ((width as f64 * s).round() as u16).max(2);
                let h = ((height as f64 * s).round() as u16).max(2);
                let rect = centered(area, w, h);
                Clear.render(rect, buf);
                Some((rect, s >= 0.95))
            }
            Frame::Tab => {
                if popup.closing_at.is_some() || area.is_empty() {
                    return None;
                }
                Clear.render(area, buf);
                Some((area, true))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_fills_its_area_at_once_and_vanishes_when_closing() {
        let area = Rect::new(0, 5, 100, 20);
        let mut buf = Buffer::empty(Rect::new(0, 0, 100, 30));
        let mut popup = Popup {
            opened_at: 10.0,
            closing_at: None,
        };
        assert_eq!(Frame::Tab.width(area, 92), 100);
        assert_eq!(Frame::Tab.height(area, 8), 20);
        assert_eq!(
            Frame::Tab.place(&mut buf, area, 10.0, &popup, 100, 20),
            Some((area, true)),
            "plein cadre dès l'ouverture"
        );
        popup.closing_at = Some(11.0);
        assert_eq!(
            Frame::Tab.place(&mut buf, area, 11.0, &popup, 100, 20),
            None
        );
    }

    #[test]
    fn a_window_grows_centred() {
        let area = Rect::new(0, 0, 100, 30);
        let mut buf = Buffer::empty(area);
        let popup = Popup {
            opened_at: 0.0,
            closing_at: None,
        };
        assert_eq!(Frame::Window.width(area, 92), 92);
        assert_eq!(Frame::Window.height(area, 40), 28);
        assert_eq!(
            Frame::Window.place(&mut buf, area, 0.0, &popup, 92, 20),
            None
        );
        let (rect, ready) = Frame::Window
            .place(&mut buf, area, 1.0, &popup, 92, 20)
            .unwrap();
        assert!(ready);
        assert_eq!(rect, Rect::new(4, 5, 92, 20));
    }
}
