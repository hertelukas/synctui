use ratatui::crossterm::event::{KeyCode, KeyEvent};

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
