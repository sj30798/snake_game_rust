mod game;

use crossterm::{cursor, execute, terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType}};
use std::io;

struct TerminalGuard;

impl TerminalGuard {
    fn setup() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, Clear(ClearType::All), cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show, cursor::MoveTo(0, 24));
        let _ = disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    let _terminal_guard = TerminalGuard::setup()?;
    let mut game = game::Game::new();
    game.run()
}
