use std::sync::{Arc, Mutex};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Position, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use super::{
    app::{CurrentMode, state::State},
    input::Message,
};

pub trait Popup: std::fmt::Debug {
    /// Updates the state of the popup. If Some(Quit) is returned, the popup gets destroyed
    fn update(&mut self, msg: Message, state: Arc<Mutex<Option<State>>>) -> Option<Message>;
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

#[derive(Default, Debug)]
struct TextBox {
    text: String,
    index: usize,
}

// This impl is heavily inspired (copied) by https://ratatui.rs/examples/apps/user_input/
impl TextBox {
    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.index.saturating_sub(1);
        self.index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.index.saturating_add(1);
        self.index = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.text.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.text
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.index)
            .unwrap_or(self.text.len())
    }

    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.text.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.text.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.text = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.text.chars().count())
    }
}

#[derive(Debug)]
pub struct NewFolderPopup {
    id_input: TextBox,
    path_input: TextBox,
    focus: NewFolderFocus,
    mode: Arc<Mutex<CurrentMode>>,
}

#[derive(Default, Debug)]
enum NewFolderFocus {
    #[default]
    Path,
    Id,
}

impl NewFolderPopup {
    pub fn new(mode: Arc<Mutex<CurrentMode>>) -> Self {
        Self {
            id_input: TextBox::default(),
            path_input: TextBox::default(),
            focus: NewFolderFocus::default(),
            mode,
        }
    }
}

impl Popup for NewFolderPopup {
    fn update(&mut self, msg: Message, _: Arc<Mutex<Option<State>>>) -> Option<Message> {
        match msg {
            Message::Quit => return Some(Message::Quit),
            Message::Character(c) => match self.focus {
                NewFolderFocus::Path => self.path_input.enter_char(c),
                NewFolderFocus::Id => self.id_input.enter_char(c),
            },
            Message::Left => match self.focus {
                NewFolderFocus::Path => self.path_input.move_cursor_left(),
                NewFolderFocus::Id => self.id_input.move_cursor_left(),
            },
            Message::Right => match self.focus {
                NewFolderFocus::Path => self.path_input.move_cursor_right(),
                NewFolderFocus::Id => self.id_input.move_cursor_right(),
            },
            Message::FocusNext | Message::Down | Message::Up => match self.focus {
                NewFolderFocus::Path => self.focus = NewFolderFocus::Id,
                NewFolderFocus::Id => self.focus = NewFolderFocus::Path,
            },
            Message::Backspace => match self.focus {
                NewFolderFocus::Id => self.id_input.delete_char(),
                NewFolderFocus::Path => self.path_input.delete_char(),
            },
            _ => {}
        };
        None
    }

    fn render(&self, frame: &mut Frame) {
        let block = self.create_popup_block("New Folder".to_string());
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
        ]);

        let area = centered_rect(50, 50, frame.area());
        let [_, path_area, id_area] = vertical.areas(area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }));

        let path_input = Paragraph::new(self.path_input.text.as_str())
            .style(match self.focus {
                NewFolderFocus::Path => Style::default().fg(Color::Blue),
                NewFolderFocus::Id => Style::default(),
            })
            .block(Block::bordered().title("Path"));

        let id_input = Paragraph::new(self.id_input.text.as_str())
            .style(match self.focus {
                // TODO check if valid (unique) and if not, make red
                NewFolderFocus::Id => Style::default().fg(Color::Blue),
                NewFolderFocus::Path => Style::default(),
            })
            .block(Block::bordered().title("ID"));

        // Show cursors
        if *self.mode.lock().unwrap() == CurrentMode::Insert {
            let (cursor_area, index) = match self.focus {
                NewFolderFocus::Path => (path_area, self.path_input.index),
                NewFolderFocus::Id => (id_area, self.id_input.index),
            };
            frame.set_cursor_position(Position::new(
                cursor_area.x + index as u16 + 1,
                cursor_area.y + 1,
            ));
        }

        frame.render_widget(block, area);
        frame.render_widget(path_input, path_area);
        frame.render_widget(id_input, id_area);
    }
}
