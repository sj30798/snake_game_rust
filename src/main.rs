mod game;

use crossterm::{cursor, execute, terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType}};
use game::ControlMode;
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
    let mode = parse_mode();
    let mut game = game::Game::new(mode);
    game.prepare_auto();

    let _terminal_guard = TerminalGuard::setup()?;
    game.run()
}

fn parse_mode() -> ControlMode {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|arg| arg == "--auto") {
        let episodes = args
            .iter()
            .position(|arg| arg == "--episodes")
            .and_then(|idx| args.get(idx + 1))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(3000);

        println!("Auto mode enabled with {} episodes", episodes);

        ControlMode::Auto { episodes }
    } else {
        ControlMode::Manual
    }
}
