//! Palette centralisée : aucune couleur en dur ailleurs dans la présentation.

use ratatui::style::{Color, Modifier, Style};

use crate::fx::color::gradient_cyclic;
use crate::fx::Rgb;

pub const BORDER: Rgb = Rgb(90, 90, 140);
pub const FOCUS: Rgb = Rgb(120, 200, 255);
pub const GOLD: Rgb = Rgb(255, 200, 80);
pub const GREEN: Rgb = Rgb(120, 230, 140);
pub const GREY: Rgb = Rgb(110, 110, 130);
pub const PINK: Rgb = Rgb(255, 120, 180);
pub const RED: Rgb = Rgb(255, 80, 90);
pub const CYAN: Rgb = Rgb(110, 230, 255);
pub const ORANGE: Rgb = Rgb(255, 150, 60);
pub const WHITE: Rgb = Rgb(255, 255, 255);
pub const TEXT: Rgb = Rgb(215, 215, 230);
pub const MUTED: Rgb = Rgb(150, 150, 175);
pub const SELECT_BG: Rgb = Rgb(30, 30, 50);
pub const BAR_EMPTY: Rgb = Rgb(55, 55, 80);
pub const BLACK: Rgb = Rgb(0, 0, 0);

/// Dégradé du logo, du titre et du gros compteur : violet → rose → or.
pub const LOGO_GRADIENT: [Rgb; 3] = [Rgb(150, 90, 255), Rgb(255, 100, 190), Rgb(255, 200, 80)];

/// Couleurs des générateurs (index = générateur).
/// Nombre d'emplacements d'effets par générateur.
pub const GEN_SLOTS: usize = 16;

pub const GENERATOR_COLORS: [Rgb; 12] = [
    Rgb(160, 200, 255),
    Rgb(120, 220, 200),
    Rgb(130, 230, 130),
    Rgb(220, 220, 140),
    Rgb(255, 190, 90),
    Rgb(255, 130, 70),
    Rgb(240, 110, 180),
    Rgb(180, 120, 255),
    Rgb(255, 255, 255),
    Rgb(200, 235, 255),
    Rgb(255, 235, 110),
    Rgb(255, 90, 150),
];

pub const NEBULA: [Rgb; 3] = [Rgb(22, 12, 42), Rgb(40, 22, 70), Rgb(28, 30, 80)];
pub const STAR_DIM: Rgb = Rgb(80, 80, 120);
pub const STAR_BRIGHT: Rgb = Rgb(235, 235, 255);
pub const SUN: [Rgb; 4] = [
    Rgb(255, 245, 190),
    Rgb(255, 210, 90),
    Rgb(255, 140, 50),
    Rgb(200, 60, 40),
];
pub const COMET_TRAIL: [Rgb; 3] = [Rgb(255, 255, 255), Rgb(120, 230, 255), Rgb(60, 90, 220)];
pub const AURORA: [Rgb; 3] = [Rgb(60, 230, 140), Rgb(60, 210, 230), Rgb(170, 90, 240)];
pub const FIREWORKS: [[Rgb; 3]; 4] = [
    [Rgb(255, 120, 180), Rgb(255, 200, 220), Rgb(255, 255, 255)],
    [Rgb(120, 230, 255), Rgb(180, 140, 255), Rgb(255, 255, 255)],
    [Rgb(255, 200, 80), Rgb(255, 140, 60), Rgb(255, 250, 200)],
    [Rgb(120, 230, 140), Rgb(200, 255, 160), Rgb(255, 255, 255)],
];
pub const SPARKS: [Rgb; 3] = [Rgb(255, 220, 120), Rgb(255, 255, 255), Rgb(255, 170, 90)];

/// Palettes des univers parallèles (index = univers) : trois teintes du vortex.
pub const UNIVERSE_PALETTES: [[Rgb; 3]; 6] = [
    [Rgb(255, 220, 120), Rgb(255, 140, 190), Rgb(150, 90, 255)],
    [Rgb(255, 120, 60), Rgb(200, 40, 40), Rgb(90, 20, 30)],
    [Rgb(150, 230, 255), Rgb(80, 140, 255), Rgb(40, 40, 140)],
    [Rgb(255, 150, 220), Rgb(200, 90, 255), Rgb(80, 40, 160)],
    [Rgb(255, 225, 150), Rgb(220, 160, 80), Rgb(120, 80, 40)],
    [Rgb(170, 255, 190), Rgb(120, 150, 200), Rgb(70, 60, 90)],
];

pub fn universe_palette(index: usize) -> [Rgb; 3] {
    UNIVERSE_PALETTES
        .get(index)
        .copied()
        .unwrap_or(UNIVERSE_PALETTES[0])
}

/// Couleur principale d'un univers.
pub fn universe_color(index: usize) -> Rgb {
    universe_palette(index)[0]
}

pub fn generator_color(index: usize) -> Rgb {
    GENERATOR_COLORS.get(index).copied().unwrap_or(WHITE)
}

/// Couleur du dégradé animé pour la colonne `col` à l'instant `t`.
pub fn logo_color(col: f64, t: f64) -> Rgb {
    gradient_cyclic(&LOGO_GRADIENT, col / 48.0 - t * 0.12)
}

pub fn fg(c: Rgb) -> Style {
    Style::default().fg(c.color())
}

pub fn bold(c: Rgb) -> Style {
    fg(c).add_modifier(Modifier::BOLD)
}

pub fn border_style(focused: bool) -> Style {
    if focused {
        bold(FOCUS)
    } else {
        fg(BORDER)
    }
}

pub fn title_style(focused: bool) -> Style {
    if focused {
        bold(FOCUS)
    } else {
        fg(MUTED).add_modifier(Modifier::BOLD)
    }
}

pub fn select_bg() -> Color {
    SELECT_BG.color()
}
