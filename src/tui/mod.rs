use std::io;
use ui::ui;

use app::App;
use color_eyre::eyre;
use ratatui::{
    Terminal,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::{Backend, CrosstermBackend},
};

use crate::Client;

mod app;
mod input;
mod ui;

pub async fn start(client: Client) -> eyre::Result<()> {
    init_panic_hook();

    // Setup terminal
    let mut terminal = init_tui()?;
    terminal.clear()?;

    let mut app = App::new(client);
    let _ = run(&mut terminal, &mut app).await;

    //restore terminal
    restore_tui()?;
    terminal.show_cursor()?;

    Ok(())
}

/// Overwrits the default panic hook by first
/// trying to restore our terminal
fn init_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Ignore errors, as we are already panicing
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

fn init_tui() -> io::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(io::stdout()))
}

fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

async fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), std::io::Error> {
    while app.running {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            // Ignore releases
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            let mut msg = Some(input::handler(key));
            while let Some(m) = msg {
                msg = app.update(m);
            }
        }
    }
    Ok(())
}
