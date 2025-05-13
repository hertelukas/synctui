use crossterm::event::KeyModifiers;
use futures::StreamExt;
use log::debug;
use ratatui::crossterm::{
    self,
    event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind},
};

use super::{app::CurrentMode, state::Folder};

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    // Vim
    Insert,
    Normal,
    // Input
    Character(char),
    Backspace,
    // Navigation
    Number(u32),
    FocusNext,
    FocusBack,
    // Movement
    Up,
    Down,
    Right,
    Left,
    // General
    Add,
    Quit,
    Reload,
    Select,
    Submit,
    // Popups
    // NewFolder
    NewFolder(Folder),
    // PendingDevice
    NewPendingDevice(String),
    AcceptDevice(String),
    IgnoreDevice(String),
    DismissDevice(String),
    // PendingFolder
    NewPendingFolder(String, String),
    None,
}

pub fn handler(key_event: KeyEvent, mode: CurrentMode) -> Message {
    if mode == CurrentMode::Normal {
        match key_event.code {
            KeyCode::Char('r') => Message::Reload,
            KeyCode::Char('q') => Message::Quit,
            KeyCode::Char('j') | KeyCode::Down => Message::Down,
            KeyCode::Char('k') | KeyCode::Up => Message::Up,
            KeyCode::Char('l') | KeyCode::Right => Message::Right,
            KeyCode::Char('h') | KeyCode::Left => Message::Left,
            KeyCode::Char('i') => Message::Insert,
            KeyCode::Char('+') | KeyCode::Char('o') => Message::Add,
            KeyCode::Enter => {
                if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                    // BUG this does not work on Linux and Mac
                    Message::Submit
                } else {
                    Message::Select
                }
            }
            KeyCode::Tab => Message::FocusNext,
            KeyCode::BackTab => Message::FocusBack,
            KeyCode::Char(a) => {
                if let Some(a) = a.to_digit(10) {
                    Message::Number(a)
                } else {
                    Message::None
                }
            }
            _ => Message::None,
        }
    } else {
        match key_event.code {
            KeyCode::Char('+') => Message::Add,
            KeyCode::Char(a) => Message::Character(a),
            KeyCode::Backspace => Message::Backspace,
            KeyCode::Down => Message::Down,
            KeyCode::Up => Message::Up,
            KeyCode::Right => Message::Right,
            KeyCode::Left => Message::Left,
            KeyCode::Esc => Message::Normal,
            KeyCode::Enter => {
                if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                    // BUG this does not work on Linux and Mac
                    Message::Submit
                } else {
                    Message::Select
                }
            }
            KeyCode::Tab => Message::FocusNext,
            KeyCode::BackTab => Message::FocusBack,
            _ => Message::None,
        }
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
                if let Some(Ok(CrosstermEvent::Key(key))) = event {
                    if key.kind == KeyEventKind::Press {
                        debug!("got key {key:?} - sending");
                        tx.send(Event::Key(key)).unwrap();
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
