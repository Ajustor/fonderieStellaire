//! Initialisation et restauration du terminal (garde RAII + panic hook).

use std::io::{self, BufWriter, Stdout};
use std::sync::Once;

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, BeginSynchronizedUpdate, EndSynchronizedUpdate,
    EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};

/// La sortie passe par un tampon : chaque image part en une seule écriture
/// (au lieu d'une écriture par ligne), ce qui évite les images à moitié
/// dessinées sur les terminaux lents.
pub type Term = Terminal<CrosstermBackend<BufWriter<Stdout>>>;

/// Taille du tampon de sortie (une image plein écran très animée fait ~10 Ko).
const OUTPUT_BUFFER: usize = 64 * 1024;

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
        let out = BufWriter::with_capacity(OUTPUT_BUFFER, stdout);
        match Terminal::new(CrosstermBackend::new(out)) {
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

/// Dessine une image en mise à jour synchronisée : le terminal l'affiche d'un
/// bloc (ignoré par les terminaux qui ne la gèrent pas).
pub fn draw_synchronized(terminal: &mut Term, render: impl FnOnce(&mut Frame)) -> io::Result<()> {
    crossterm::queue!(terminal.backend_mut(), BeginSynchronizedUpdate)?;
    terminal.draw(render)?;
    crossterm::execute!(terminal.backend_mut(), EndSynchronizedUpdate)?;
    Ok(())
}
