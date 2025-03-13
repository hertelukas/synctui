use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
};
use strum::IntoEnumIterator;

use crate::app::{App, CurrentScreen};

pub fn ui(frame: &mut Frame, app: &App) {
    frame.render_widget(create_background(app), frame.area());
}

fn create_background(app: &App) -> Block {
    let block = Block::default()
        .title_top(Line::from("| SyncTUI |").centered())
        .borders(Borders::ALL);

    let mut bottom_string = CurrentScreen::iter()
        .enumerate()
        .map(|(i, screen)| {
            Span::styled(
                format!("| ({}) {:?} ", i + 1, screen),
                if screen == app.current_screen {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            )
        })
        .collect::<Vec<Span>>();
    bottom_string.push("|".into());

    block
        .title_bottom(bottom_string)
        .title_bottom(Line::from("| (q) quit |").right_aligned())
}
