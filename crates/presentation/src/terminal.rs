//! Initialisation et restauration du terminal (garde RAII + panic hook).

use std::io::{self, Stdout};
use std::sync::Once;

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub type Term = Terminal<CrosstermBackend<Stdout>>;

static PANIC_HOOK: Once = Once::new();

/// Remet le terminal dans son état normal. Sans effet si déjà fait.
pub fn restore() -> io::Result<()> {
    let raw = disable_raw_mode();
    let screen = execute!(io::stdout(), LeaveAlternateScreen, Show);
    raw.and(screen)
}

/// Installe (une seule fois) un hook qui restaure le terminal avant d'afficher un panic.
pub fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = restore();
            previous(info);
        }));
    });
}

/// Possède le terminal et le restaure quand elle est détruite (sortie normale ou erreur).
pub struct TerminalGuard {
    terminal: Term,
}

impl TerminalGuard {
    pub fn new() -> io::Result<Self> {
        install_panic_hook();
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(e) = execute!(stdout, EnterAlternateScreen, Hide) {
            let _ = restore();
            return Err(e);
        }
        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(mut terminal) => {
                let _ = terminal.clear();
                Ok(TerminalGuard { terminal })
            }
            Err(e) => {
                let _ = restore();
                Err(e)
            }
        }
    }

    pub fn terminal(&mut self) -> &mut Term {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore();
    }
}
