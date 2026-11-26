//! Couche présentation : terminal (ratatui + crossterm), effets visuels, écrans.
//!
//! Ne modifie jamais l'état du jeu directement : envoie des `Command` au
//! `GameService` et lit un `GameSnapshot`. Les états purement visuels
//! (tweens, particules, défilement, focus) vivent uniquement ici.

pub mod app;
pub mod bigfont;
pub mod draw;
pub mod fmt;
pub mod fx;
pub mod input;
pub mod layout;
pub mod screens;
pub mod terminal;
pub mod theme;
pub mod ui;
pub mod visuals;
pub mod widgets;

pub use app::{run, App, RunOutcome, Screen};
