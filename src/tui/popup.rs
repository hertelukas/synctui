use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::ty::AddedPendingDevice;

use super::{
    app::{CurrentMode, state::State},
    input::Message,
};

pub trait Popup: std::fmt::Debug {
    /// Updates the state of the popup. If Some(Quit) is returned, the popup gets destroyed
    fn update(&mut self, msg: Message, state: Arc<Mutex<State>>) -> Option<Message>;
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
    label_input: TextBox,
    path_input: TextBox,
    focus: NewFolderFocus,
    mode: Arc<Mutex<CurrentMode>>,
    state: Arc<Mutex<State>>,
    selected_devices: HashSet<String>,
}

#[derive(Default, Debug, PartialEq, Eq)]
enum NewFolderFocus {
    #[default]
    Path,
    Label,
    Id,
    Device(usize),
    SubmitButton,
}

impl NewFolderFocus {
    fn is_input(&self) -> bool {
        match self {
            Self::Device(_) | Self::SubmitButton => false,
            _ => true,
        }
    }
}

impl NewFolderPopup {
    pub fn new(mode: Arc<Mutex<CurrentMode>>, state: Arc<Mutex<State>>) -> Self {
        Self {
            id_input: TextBox::default(),
            label_input: TextBox::default(),
            path_input: TextBox::default(),
            focus: NewFolderFocus::default(),
            mode,
            state,
            selected_devices: HashSet::new(),
        }
    }

    fn select_next(&mut self) {
        let devices_len = self.state.lock().unwrap().get_other_devices().len();
        match self.focus {
            NewFolderFocus::Path => self.focus = NewFolderFocus::Label,
            NewFolderFocus::Label => self.focus = NewFolderFocus::Id,
            NewFolderFocus::Id => {
                if devices_len > 0 {
                    self.focus = NewFolderFocus::Device(0);
                } else {
                    self.focus = NewFolderFocus::SubmitButton;
                }
            }
            NewFolderFocus::Device(i) => {
                if i + 1 < devices_len {
                    self.focus = NewFolderFocus::Device(i + 1);
                } else {
                    self.focus = NewFolderFocus::SubmitButton;
                }
            }
            _ => {}
        };
    }

    fn select_prev(&mut self) {
        match self.focus {
            NewFolderFocus::Id => self.focus = NewFolderFocus::Label,
            NewFolderFocus::Label => self.focus = NewFolderFocus::Path,
            NewFolderFocus::Device(i) => {
                if i == 0 {
                    self.focus = NewFolderFocus::Id;
                } else {
                    self.focus = NewFolderFocus::Device(i - 1);
                }
            }
            NewFolderFocus::SubmitButton => {
                let devices_len = self.state.lock().unwrap().get_other_devices().len();
                if devices_len > 0 {
                    self.focus = NewFolderFocus::Device(devices_len - 1);
                } else {
                    self.focus = NewFolderFocus::Id;
                }
            }
            _ => {}
        };
    }
    fn submit(&mut self) -> Option<Message> {
        *self.mode.lock().unwrap() = CurrentMode::Normal;
        let folder_devices: Vec<_> = {
            let devices = &self.state.lock().unwrap().devices;
            self.selected_devices
                .iter()
                .filter_map(|device_id| devices.get(device_id).map(|device| device.into()))
                .collect()
        };

        return Some(Message::NewFolder(crate::ty::Folder::new(
            self.id_input.text.clone(),
            self.label_input.text.clone(),
            self.path_input.text.clone(),
            folder_devices,
        )));
    }
}

impl Popup for NewFolderPopup {
    fn update(&mut self, msg: Message, _: Arc<Mutex<State>>) -> Option<Message> {
        let input = match self.focus {
            NewFolderFocus::Id => Some(&mut self.id_input),
            NewFolderFocus::Label => Some(&mut self.label_input),
            NewFolderFocus::Path => Some(&mut self.path_input),
            _ => None,
        };

        if let Some(input) = input {
            match msg {
                Message::Character(c) => input.enter_char(c),
                Message::Backspace => input.delete_char(),
                Message::Left => input.move_cursor_left(),
                Message::Right => input.move_cursor_right(),
                _ => {}
            }
        }

        match msg {
            Message::Quit => return Some(Message::Quit),
            Message::FocusNext | Message::Down => self.select_next(),
            Message::FocusBack | Message::Up => self.select_prev(),
            Message::Left => {
                if let NewFolderFocus::Device(i) = self.focus {
                    if i > 0 {
                        self.select_prev();
                    }
                }
            }
            Message::Right => {
                if let NewFolderFocus::Device(_) = self.focus {
                    self.select_next();
                }
            }
            Message::Select => match self.focus {
                NewFolderFocus::SubmitButton => return self.submit(),
                NewFolderFocus::Device(i) => {
                    if let Some(device) = self.state.lock().unwrap().get_other_devices().get(i) {
                        if self.selected_devices.contains(&device.id) {
                            self.selected_devices.remove(&device.id);
                        } else {
                            self.selected_devices.insert(device.id.clone());
                        }
                    }
                }
                _ => self.select_next(),
            },
            Message::Submit => return self.submit(),
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
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(1),
        ]);

        let area = centered_rect(50, 50, frame.area());
        Clear.render(area, frame.buffer_mut());
        let [_, path_area, label_area, id_area, devices_area, submit_area] =
            vertical.areas(area.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }));

        let path_input = Paragraph::new(self.path_input.text.as_str())
            .style(match self.focus {
                NewFolderFocus::Path => Style::default().fg(Color::Blue),
                _ => Style::default(),
            })
            .block(Block::bordered().title("Path"));

        let label_input = Paragraph::new(self.label_input.text.as_str())
            .style(match self.focus {
                NewFolderFocus::Label => Style::default().fg(Color::Blue),
                _ => Style::default(),
            })
            .block(Block::bordered().title("Label"));

        let id_input = Paragraph::new(self.id_input.text.as_str())
            .style(match self.focus {
                // TODO check if valid (unique) and if not, make red
                NewFolderFocus::Id => Style::default().fg(Color::Blue),
                _ => Style::default(),
            })
            .block(Block::bordered().title("ID"));

        let devices_line: Line = self
            .state
            .lock()
            .unwrap()
            .get_other_devices()
            .iter()
            .enumerate()
            .map(|(i, device)| {
                let style = if self.focus == NewFolderFocus::Device(i) {
                    Style::new().fg(Color::Blue)
                } else {
                    Style::new()
                };
                let selected_char = if self.selected_devices.contains(&device.id) {
                    "✓"
                } else {
                    "☐"
                };
                Span::styled(
                    format!("| {} {} ", selected_char, device.name.clone()),
                    style,
                )
            })
            .collect();

        let devices_select = Paragraph::new(devices_line);

        let submit = Paragraph::new(Span::styled(
            "Submit",
            match self.focus {
                NewFolderFocus::SubmitButton => Style::default().bg(Color::DarkGray),
                _ => Style::default(),
            },
        ));

        // Show cursors
        if *self.mode.lock().unwrap() == CurrentMode::Insert {
            let (cursor_area, index) = match self.focus {
                NewFolderFocus::Path => (path_area, self.path_input.index),
                NewFolderFocus::Id => (id_area, self.id_input.index),
                NewFolderFocus::Label => (label_area, self.label_input.index),
                _ => (area, 0),
            };
            if self.focus.is_input() {
                frame.set_cursor_position(Position::new(
                    cursor_area.x + index as u16 + 1,
                    cursor_area.y + 1,
                ));
            }
        }

        frame.render_widget(block, area);
        frame.render_widget(path_input, path_area);
        frame.render_widget(label_input, label_area);
        frame.render_widget(id_input, id_area);
        frame.render_widget(devices_select, devices_area);
        frame.render_widget(submit, submit_area);
    }
}

#[derive(Debug)]
pub struct PendingDevicePopup {
    device: AddedPendingDevice,
    focus: PendingDeviceFocus,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum PendingDeviceFocus {
    #[default]
    Accept,
    Ignore,
    Dismiss,
}

impl PendingDevicePopup {
    pub fn new(device: AddedPendingDevice) -> Self {
        Self {
            device,
            focus: PendingDeviceFocus::default(),
        }
    }

    fn select_next(&mut self) {
        match self.focus {
            PendingDeviceFocus::Accept => self.focus = PendingDeviceFocus::Ignore,
            PendingDeviceFocus::Ignore => self.focus = PendingDeviceFocus::Dismiss,
            PendingDeviceFocus::Dismiss => {}
        }
    }

    fn select_prev(&mut self) {
        match self.focus {
            PendingDeviceFocus::Accept => {}
            PendingDeviceFocus::Ignore => self.focus = PendingDeviceFocus::Accept,
            PendingDeviceFocus::Dismiss => self.focus = PendingDeviceFocus::Ignore,
        }
    }
    fn submit(&self) -> Option<Message> {
        match self.focus {
            PendingDeviceFocus::Accept => {
                Some(Message::AcceptDevice(self.device.device_id.clone()))
            }
            PendingDeviceFocus::Ignore => {
                Some(Message::IgnoreDevice(self.device.device_id.clone()))
            }
            PendingDeviceFocus::Dismiss => {
                Some(Message::DismissDevice(self.device.device_id.clone()))
            }
        }
    }
}

impl Popup for PendingDevicePopup {
    fn update(&mut self, msg: Message, _state: Arc<Mutex<State>>) -> Option<Message> {
        match msg {
            Message::Quit => return Some(Message::Quit),
            Message::FocusNext | Message::Right => self.select_next(),
            Message::FocusBack | Message::Left => self.select_prev(),
            Message::Select | Message::Submit => return self.submit(),
            _ => {}
        };
        None
    }

    fn render(&self, frame: &mut Frame) {
        let block = self.create_popup_block("Pending Device".to_string());
        let vertical = Layout::vertical([Constraint::Length(2), Constraint::Length(1)]);

        let area = centered_rect(50, 50, frame.area());
        Clear.render(area, frame.buffer_mut());
        let [message_area, buttons_area] = vertical.areas(area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }));
        let line = Line::from(format!("Device {} wants to connect.", self.device));

        let selected_style = Style::new().bg(Color::DarkGray);

        let buttons_line: Line = vec![
            Span::styled(
                "Accept",
                if matches!(self.focus, PendingDeviceFocus::Accept) {
                    selected_style
                } else {
                    Style::new()
                },
            ),
            Span::raw(" "),
            Span::styled(
                "Ignore",
                if matches!(self.focus, PendingDeviceFocus::Ignore) {
                    selected_style
                } else {
                    Style::new()
                },
            ),
            Span::raw(" "),
            Span::styled(
                "Dismiss",
                if matches!(self.focus, PendingDeviceFocus::Dismiss) {
                    selected_style
                } else {
                    Style::new()
                },
            ),
        ]
        .into();

        frame.render_widget(block, area);
        frame.render_widget(line, message_area);
        frame.render_widget(buttons_line, buttons_area);
    }
}
