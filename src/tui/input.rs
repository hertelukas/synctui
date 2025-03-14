use futures::StreamExt;
use log::debug;
use ratatui::crossterm::{
    self,
    event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind},
};

#[derive(Debug, PartialEq)]
pub enum Message {
    // Navigation
    Number(u32),
    // Movement
    Up,
    Down,
    Right,
    Left,
    // General
    Quit,
    None,
}

pub fn handler(key_event: KeyEvent) -> Message {
    match key_event.code {
        KeyCode::Char('q') => Message::Quit,
        KeyCode::Char('j') | KeyCode::Down => Message::Down,
        KeyCode::Char('k') | KeyCode::Up => Message::Up,
        KeyCode::Char('l') | KeyCode::Right => Message::Right,
        KeyCode::Char('h') | KeyCode::Left => Message::Left,
        KeyCode::Char(a) => {
            if let Some(a) = a.to_digit(10) {
                Message::Number(a)
            } else {
                Message::None
            }
        }
        _ => Message::None,
    }
}

#[derive(Debug)]
pub enum Event {
    Key(crossterm::event::KeyEvent),
}

pub struct EventHandler {
    rx: tokio::sync::mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let _tx = tx.clone();
        tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            loop {
                let event = reader.next().await;
                if let Some(Ok(event)) = event {
                    match event {
                        CrosstermEvent::Key(key) => {
                            if key.kind == KeyEventKind::Press {
                                debug!("Got key {key:?} - sending");
                                tx.send(Event::Key(key)).unwrap();
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
        EventHandler { rx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
