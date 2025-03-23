use std::sync::{Arc, Mutex};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders},
};

use super::{app::state::State, input::Message};

pub trait Popup: std::fmt::Debug {
    /// Updates the state of the popup. If Some(Quit) is returned, the popup gets destroyed
    fn update(&self, msg: Message, state: Arc<Mutex<Option<State>>>) -> Option<Message>;
    fn render(&self, frame: &mut Frame);
    fn create_popup_block(&self, title: String) -> Block {
        let block = Block::default()
            .title_top(Line::from(format!("| {} |", title)).centered())
            .borders(Borders::ALL);

        block
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
// Adapted from https://ratatui.rs/tutorials/json-editor/ui/
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}

#[derive(Debug)]
pub struct NewFolderPopup {}

impl Popup for NewFolderPopup {
    fn update(&self, msg: Message, _: Arc<Mutex<Option<State>>>) -> Option<Message> {
        match msg {
            Message::Quit => return Some(Message::Quit),
            _ => return None,
        }
    }

    fn render(&self, frame: &mut Frame) {
        let block = self.create_popup_block("New Folder".to_string());

        let area = centered_rect(50, 50, frame.area());
        frame.render_widget(block, area);
    }
}

impl NewFolderPopup {
    pub fn new() -> Self {
        Self {}
    }
}
