use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListState, Paragraph, StatefulWidget, Widget},
};
use strum::IntoEnumIterator;
use syncthing_rs::types::config::{
    FolderConfiguration, FolderDeviceConfiguration, NewFolderConfiguration,
};

use super::{app::CurrentMode, input::Message};

use crate::tui::state::State;

pub trait Popup: std::fmt::Debug {
    /// Updates the state of the popup. If Some(Quit) is returned, the popup gets destroyed
    fn update(&mut self, msg: Message, state: State) -> Option<Message>;
    fn render(&self, frame: &mut Frame, state: State);
    fn create_popup_block(&self, title: String) -> Block {
        Block::default()
            .title_top(Line::from(format!("| {} |", title)).centered().bold())
            .borders(Borders::ALL)
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

    fn as_paragraph<'a>(&'a self, title: &'a str, style: Style) -> Paragraph<'a> {
        Paragraph::new(self.text.as_str())
            .style(style)
            .block(Block::bordered().title(title))
    }
}

impl From<String> for TextBox {
    fn from(value: String) -> Self {
        let index = value.chars().count();
        Self { text: value, index }
    }
}

#[derive(Debug)]
pub struct NewFolderPopup {
    id_input: TextBox,
    label_input: TextBox,
    path_input: TextBox,
    focus: NewFolderFocus,
    mode: Arc<Mutex<CurrentMode>>,
    state: State,
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
        !matches!(self, Self::Device(_) | Self::SubmitButton)
    }
}

impl NewFolderPopup {
    pub fn new(mode: Arc<Mutex<CurrentMode>>, state: State) -> Self {
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

    /// This can be used if accepting a folder from another device
    pub fn new_from_device(
        folder_label: impl Into<String>,
        folder_id: impl Into<String>,
        device_id: impl Into<String>,
        mode: Arc<Mutex<CurrentMode>>,
        state: State,
    ) -> Self {
        let mut selected_devices = HashSet::new();
        selected_devices.insert(device_id.into());
        Self {
            id_input: folder_id.into().into(),
            label_input: folder_label.into().into(),
            path_input: TextBox::default(),
            focus: NewFolderFocus::default(),
            mode,
            state,
            selected_devices,
        }
    }

    fn select_next(&mut self) {
        let devices_len = self.state.read(|state| state.get_other_devices().len());
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
                let devices_len = self.state.read(|state| state.get_other_devices().len());
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
        let devices: Vec<FolderDeviceConfiguration> = self
            .selected_devices
            .iter()
            .map(|d| FolderDeviceConfiguration {
                device_id: d.to_string(),
                introduced_by: "".to_string(),
                encryption_password: "".to_string(),
            })
            .collect();
        Some(Message::NewFolder(Box::new(
            NewFolderConfiguration::new(self.id_input.text.clone(), self.path_input.text.clone())
                .label(self.label_input.text.clone())
                .devices(devices),
        )))
    }
}

impl Popup for NewFolderPopup {
    fn update(&mut self, msg: Message, _: State) -> Option<Message> {
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
                    if let Some(device_id) = self.state.read(|state| {
                        state
                            .get_other_devices()
                            .get(i)
                            .map(|d| d.config.device_id.clone())
                    }) {
                        if self.selected_devices.contains(&device_id) {
                            self.selected_devices.remove(&device_id);
                        } else {
                            self.selected_devices.insert(device_id);
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

    fn render(&self, frame: &mut Frame, _state: State) {
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

        let devices_line: Line = self.state.read(|state| {
            state
                .get_other_devices()
                .iter()
                .enumerate()
                .map(|(i, device)| {
                    let style = if self.focus == NewFolderFocus::Device(i) {
                        Style::new().fg(Color::Blue)
                    } else {
                        Style::new()
                    };
                    let selected_char = if self.selected_devices.contains(&device.config.device_id)
                    {
                        "✓"
                    } else {
                        "☐"
                    };
                    Span::styled(
                        format!("| {} {} ", selected_char, device.config.name.clone()),
                        style,
                    )
                })
                .collect()
        });

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
    device_id: String,
    focus: PendingFocus,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum PendingFocus {
    #[default]
    Accept,
    Ignore,
    Dismiss,
}

impl PendingFocus {
    fn next(&mut self) {
        match self {
            PendingFocus::Accept => *self = PendingFocus::Ignore,
            PendingFocus::Ignore => *self = PendingFocus::Dismiss,
            PendingFocus::Dismiss => {}
        }
    }

    fn prev(&mut self) {
        match self {
            PendingFocus::Accept => {}
            PendingFocus::Ignore => *self = PendingFocus::Accept,
            PendingFocus::Dismiss => *self = PendingFocus::Ignore,
        }
    }
}

impl PendingDevicePopup {
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            focus: PendingFocus::default(),
        }
    }

    fn submit(&self) -> Option<Message> {
        match self.focus {
            PendingFocus::Accept => Some(Message::AcceptDevice(self.device_id.clone())),
            PendingFocus::Ignore => Some(Message::IgnoreDevice(self.device_id.clone())),
            PendingFocus::Dismiss => Some(Message::DismissDevice(self.device_id.clone())),
        }
    }
}

impl Popup for PendingDevicePopup {
    fn update(&mut self, msg: Message, _state: State) -> Option<Message> {
        match msg {
            Message::Quit => return Some(Message::Quit),
            Message::FocusNext | Message::Right => self.focus.next(),
            Message::FocusBack | Message::Left => self.focus.prev(),
            Message::Select | Message::Submit => return self.submit(),
            _ => {}
        };
        None
    }

    fn render(&self, frame: &mut Frame, _state: State) {
        let block = self.create_popup_block("Pending Device".to_string());
        let vertical = Layout::vertical([Constraint::Length(2), Constraint::Length(1)]);

        let area = centered_rect(50, 50, frame.area());
        Clear.render(area, frame.buffer_mut());
        let [message_area, buttons_area] = vertical.areas(area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }));
        // TODO use state to load device name
        let line = Line::from(format!("Device {} wants to connect.", self.device_id));

        let selected_style = Style::new().bg(Color::DarkGray);

        let buttons_line: Line = vec![
            Span::styled(
                "Accept",
                if matches!(self.focus, PendingFocus::Accept) {
                    selected_style
                } else {
                    Style::new()
                },
            ),
            Span::raw(" "),
            Span::styled(
                "Ignore",
                if matches!(self.focus, PendingFocus::Ignore) {
                    selected_style
                } else {
                    Style::new()
                },
            ),
            Span::raw(" "),
            Span::styled(
                "Dismiss",
                if matches!(self.focus, PendingFocus::Dismiss) {
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

/// Popup to share an already existing folder with a new device
#[derive(Debug)]
pub struct PendingShareFolderPopup {
    folder_id: String,
    device_id: String,
    focus: PendingFocus,
}

impl PendingShareFolderPopup {
    pub fn new(folder_id: String, device_id: String) -> Self {
        Self {
            folder_id,
            device_id,
            focus: PendingFocus::default(),
        }
    }

    fn submit(&self) -> Option<Message> {
        match self.focus {
            PendingFocus::Accept => Some(Message::ShareFolder {
                folder_id: self.folder_id.clone(),
                device_id: self.device_id.clone(),
            }),
            PendingFocus::Ignore => todo!(),
            PendingFocus::Dismiss => Some(Message::DismissFolder {
                folder_id: self.folder_id.clone(),
                device_id: self.device_id.clone(),
            }),
        }
    }
}

impl Popup for PendingShareFolderPopup {
    fn update(&mut self, msg: Message, _state: State) -> Option<Message> {
        match msg {
            Message::Quit => return Some(Message::Quit),
            Message::FocusNext | Message::Right => self.focus.next(),
            Message::FocusBack | Message::Left => self.focus.prev(),
            Message::Select | Message::Submit => return self.submit(),
            _ => {}
        };
        None
    }

    fn render(&self, frame: &mut Frame, state: State) {
        let block = self.create_popup_block("Share Folder".to_string());
        let vertical = Layout::vertical([Constraint::Length(2), Constraint::Length(1)]);

        let area = centered_rect(50, 50, frame.area());
        Clear.render(area, frame.buffer_mut());
        let [message_area, buttons_area] = vertical.areas(area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }));
        let line = state.read(|state| {
            // TODO maybe show device label too
            let folder = state
                .get_folder(&self.folder_id)
                .expect("folder to be shared does not exist on this device");
            Line::from(format!(
                "Share {} ({}) with {}",
                folder.config.label, folder.config.id, self.device_id
            ))
        });
        let selected_style = Style::new().bg(Color::DarkGray);

        let buttons_line: Line = vec![
            Span::styled(
                "Share",
                if matches!(self.focus, PendingFocus::Accept) {
                    selected_style
                } else {
                    Style::new()
                },
            ),
            Span::raw(" "),
            Span::styled(
                "Ignore",
                if matches!(self.focus, PendingFocus::Ignore) {
                    selected_style
                } else {
                    Style::new()
                },
            ),
            Span::raw(" "),
            Span::styled(
                "Dismiss",
                if matches!(self.focus, PendingFocus::Dismiss) {
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

/// Popup representing a folder
#[derive(Debug)]
pub struct FolderPopup {
    folder: FolderConfiguration,
    id: TextBox,
    label: TextBox,
    path: TextBox,
    devices: Vec<String>,
    selected_device: Option<usize>,
    focus: FolderFocus,
    general_focus: FolderGeneralFocus,
    mode: Arc<Mutex<CurrentMode>>,
}

#[derive(Debug, Default, strum::EnumIter, PartialEq, Eq)]
enum FolderFocus {
    #[default]
    General,
    Sharing,
}

impl TryFrom<u32> for FolderFocus {
    type Error = ();

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        if let Some((_, screen)) = FolderFocus::iter()
            .enumerate()
            .find(|(i, _)| i + 1 == (v as usize))
        {
            Ok(screen)
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
enum FolderGeneralFocus {
    #[default]
    Label,
    ID,
    Path,
    Submit,
}

impl FolderGeneralFocus {
    fn next(&mut self) {
        match self {
            FolderGeneralFocus::Label => *self = FolderGeneralFocus::ID,
            FolderGeneralFocus::ID => *self = FolderGeneralFocus::Path,
            FolderGeneralFocus::Path => *self = FolderGeneralFocus::Submit,
            FolderGeneralFocus::Submit => {}
        }
    }

    fn prev(&mut self) {
        match self {
            FolderGeneralFocus::Label => {}
            FolderGeneralFocus::ID => *self = FolderGeneralFocus::Label,
            FolderGeneralFocus::Path => *self = FolderGeneralFocus::ID,
            FolderGeneralFocus::Submit => *self = FolderGeneralFocus::Path,
        }
    }
}

impl FolderPopup {
    pub fn new(folder: FolderConfiguration, mode: Arc<Mutex<CurrentMode>>) -> Self {
        let devices = folder.devices.iter().map(|f| f.device_id.clone()).collect();
        Self {
            folder: folder.clone(),
            id: folder.id.into(),
            label: folder.label.into(),
            path: folder.path.into(),
            devices,
            selected_device: None,
            focus: FolderFocus::default(),
            general_focus: FolderGeneralFocus::default(),
            mode,
        }
    }

    fn submit(&self) -> Option<Message> {
        todo!()
    }
}

impl Popup for FolderPopup {
    fn update(&mut self, msg: Message, state: State) -> Option<Message> {
        match msg {
            Message::Quit => return Some(Message::Quit),
            Message::Number(i) => {
                if let Ok(focus) = FolderFocus::try_from(i) {
                    self.focus = focus;
                }
            }
            _ => {}
        }

        match self.focus {
            FolderFocus::General => {
                let input = match self.general_focus {
                    FolderGeneralFocus::Label => Some(&mut self.label),
                    FolderGeneralFocus::ID => Some(&mut self.id),
                    FolderGeneralFocus::Path => Some(&mut self.path),
                    FolderGeneralFocus::Submit => None,
                };

                match msg {
                    Message::FocusNext | Message::Down => self.general_focus.next(),
                    Message::FocusBack | Message::Up => self.general_focus.prev(),
                    Message::Character(c) => {
                        if let Some(input) = input {
                            input.enter_char(c);
                        }
                    }
                    Message::Backspace => {
                        if let Some(input) = input {
                            input.delete_char();
                        }
                    }
                    Message::Left => {
                        if let Some(input) = input {
                            input.move_cursor_left();
                        }
                    }
                    Message::Right => {
                        if let Some(input) = input {
                            input.move_cursor_right();
                        }
                    }
                    Message::Select => return self.submit(),
                    _ => {}
                }
            }
            FolderFocus::Sharing => {
                let len = state.read(|state| state.get_other_devices().len());
                match msg {
                    Message::FocusNext | Message::Down => {
                        if len == 0 {
                            return None;
                        }
                        if let Some(selected_device) = self.selected_device {
                            self.selected_device = Some((selected_device + 1) % len);
                        } else {
                            self.selected_device = Some(0)
                        }
                    }
                    Message::FocusBack | Message::Up => {
                        if len == 0 {
                            return None;
                        }
                        if let Some(selected_device) = self.selected_device {
                            self.selected_device = Some((selected_device + len - 1) % len);
                        } else {
                            self.selected_device = Some(len - 1)
                        }
                    }
                    Message::Select => {
                        if let Some(selected_device) = self.selected_device {
                            if let Some(selected_device_id) = state.read(|state| {
                                state
                                    .get_other_devices()
                                    .get(selected_device)
                                    .map(|device| device.config.device_id.clone())
                            }) {
                                match self.devices.iter().position(|d| d == &selected_device_id) {
                                    Some(index) => {
                                        self.devices.remove(index);
                                    }
                                    None => self.devices.push(selected_device_id),
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    fn render(&self, frame: &mut Frame, state: State) {
        let block = self.create_popup_block(format!("Edit Folder ({})", self.folder.label));

        let mut bottom_string = FolderFocus::iter()
            .enumerate()
            .map(|(i, focus)| {
                Span::styled(
                    format!("| ({}) {:?} ", i + 1, focus),
                    if focus == self.focus {
                        Style::default().bold()
                    } else {
                        Style::default()
                    },
                )
            })
            .collect::<Vec<Span>>();
        bottom_string.push("|".into());
        let block = block.title_bottom(bottom_string);

        let area = centered_rect(75, 75, frame.area());
        Clear.render(area, frame.buffer_mut());

        match self.focus {
            FolderFocus::General => {
                let vertical = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ]);
                let [label_area, id_area, path_area, submit_area] =
                    vertical.areas(area.inner(Margin {
                        horizontal: 2,
                        vertical: 2,
                    }));

                let focused_style = Style::default().fg(Color::Blue);

                let label_paragraph = self.label.as_paragraph(
                    "Label",
                    if self.general_focus == FolderGeneralFocus::Label {
                        focused_style
                    } else {
                        Style::default()
                    },
                );

                let id_paragraph = self.id.as_paragraph(
                    "ID",
                    if self.general_focus == FolderGeneralFocus::ID {
                        focused_style
                    } else {
                        Style::default()
                    },
                );

                let path_paragraph = self.path.as_paragraph(
                    "Path",
                    if self.general_focus == FolderGeneralFocus::Path {
                        focused_style
                    } else {
                        Style::default()
                    },
                );

                let submit = Paragraph::new(Span::styled(
                    "Submit",
                    match self.general_focus {
                        FolderGeneralFocus::Submit => Style::default().bg(Color::DarkGray),
                        _ => Style::default(),
                    },
                ));

                // Show cursor

                if *self.mode.lock().unwrap() == CurrentMode::Insert {
                    let (cursor_area, index) = match self.general_focus {
                        FolderGeneralFocus::Label => (label_area, self.label.index),
                        FolderGeneralFocus::ID => (id_area, self.id.index),
                        FolderGeneralFocus::Path => (path_area, self.path.index),
                        _ => (area, 0),
                    };
                    if self.general_focus != FolderGeneralFocus::Submit {
                        frame.set_cursor_position(Position::new(
                            cursor_area.x + index as u16 + 1,
                            cursor_area.y + 1,
                        ));
                    }
                }

                frame.render_widget(label_paragraph, label_area);
                frame.render_widget(id_paragraph, id_area);
                frame.render_widget(path_paragraph, path_area);
                frame.render_widget(submit, submit_area);
            }
            FolderFocus::Sharing => state.read(|state| {
                let lines: Vec<_> = state
                    .get_other_devices()
                    .iter()
                    .map(|device| {
                        let selected_char =
                            if self.devices.iter().any(|d| d == &device.config.device_id) {
                                "✓"
                            } else {
                                "☐"
                            };
                        Span::raw(format!("{} {}", selected_char, device.config.name))
                    })
                    .collect();

                let list = List::new(lines).highlight_style(Style::new().bg(Color::DarkGray));
                let mut list_state = ListState::default().with_selected(self.selected_device);

                let area = area.inner(Margin {
                    horizontal: 2,
                    vertical: 2,
                });

                StatefulWidget::render(list, area, frame.buffer_mut(), &mut list_state);
            }),
        }

        frame.render_widget(block, area);
    }
}
