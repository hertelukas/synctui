use input::EventHandler;
use log::debug;
use std::io;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use ui::ui;

use app::{App, CurrentMode};
use color_eyre::eyre;
use ratatui::{
    Terminal,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
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

    let (reload_tx, reload_rx) = mpsc::unbounded_channel();

    let mut app = App::new(client, reload_tx);
    let _ = run(&mut terminal, &mut app, reload_rx).await;

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

async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mut reload_rx: UnboundedReceiver<()>,
) -> Result<(), std::io::Error> {
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();

    let mode_handle = app.mode.clone();

    tokio::spawn(async move {
        let mut event = EventHandler::new();
        loop {
            let event = event.next().await;
            if let Some(input::Event::Key(k)) = event {
                let mode: CurrentMode = { mode_handle.lock().unwrap().clone() };
                msg_tx.send(input::handler(k, mode)).unwrap()
            };
        }
    });

    while app.running {
        debug!("drawing new frame");
        terminal.draw(|f| ui(f, app))?;

        tokio::select! {
            mut msg = msg_rx.recv() =>  {
                while let Some(m) = msg {
                    msg = app.update(m);
                }
            },
            _ = reload_rx.recv() => {}
        }
    }
    Ok(())
}
