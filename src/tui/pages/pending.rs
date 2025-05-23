use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListState, StatefulWidget},
};

use ratatui::widgets::Widget;

use crate::tui::{app::App, input::Message};

pub struct PendingPage<'a> {
    app: &'a App,
}

#[derive(Debug)]
pub struct PendingPageState {
    devices_focused: bool,
    focused_device: Option<usize>,
    focused_folder: Option<usize>,
}

impl Default for PendingPageState {
    fn default() -> Self {
        Self {
            devices_focused: true,
            focused_device: Default::default(),
            focused_folder: Default::default(),
        }
    }
}

impl PendingPageState {
    pub fn device_selected(&self) -> Option<usize> {
        if self.devices_focused {
            self.focused_device
        } else {
            None
        }
    }

    pub fn folder_selected(&self) -> Option<usize> {
        if !self.devices_focused {
            self.focused_folder
        } else {
            None
        }
    }

    pub fn update(&mut self, msg: &Message, total_devices: usize, total_folders: usize) {
        match msg {
            Message::Left | Message::Right | Message::FocusNext | Message::FocusBack => {
                self.devices_focused = !self.devices_focused;
            }
            _ => {}
        }

        // Focus nothing if we have no pending
        if total_devices == 0 && total_folders == 0 {
            self.focused_device = None;
            self.focused_folder = None;
            return;
        }
        // Force focus if one is 0
        if total_devices == 0 {
            self.devices_focused = false;
        }
        if total_folders == 0 {
            self.devices_focused = true;
        }

        match msg {
            Message::Down => {
                if self.devices_focused {
                    if let Some(i) = self.focused_device {
                        self.focused_device = Some((i + 1) % total_devices);
                    } else {
                        self.focused_device = Some(0);
                        self.focused_folder = Some(self.focused_folder.unwrap_or(0))
                    }
                } else if let Some(i) = self.focused_folder {
                    self.focused_folder = Some((i + 1) % total_folders);
                } else {
                    self.focused_folder = Some(0);
                    self.focused_device = Some(self.focused_device.unwrap_or(0))
                }
            }
            Message::Up => {
                if self.devices_focused {
                    if let Some(i) = self.focused_device {
                        self.focused_device = Some((i + total_devices - 1) % total_devices);
                    } else {
                        self.focused_device = Some(total_devices - 1);
                        self.focused_folder =
                            Some(self.focused_folder.unwrap_or(total_folders - 1));
                    }
                } else if let Some(i) = self.focused_folder {
                    self.focused_folder = Some((i + total_folders - 1) % total_folders);
                } else {
                    self.focused_folder = Some(total_folders - 1);
                    self.focused_device = Some(self.focused_device.unwrap_or(total_devices - 1));
                }
            }
            _ => {}
        }
    }
}

impl<'a> PendingPage<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl Widget for PendingPage<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        (&self).render(area, buf);
    }
}

impl Widget for &PendingPage<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Devices
        let devices_list: Vec<_> = self.app.state.read(|state| {
            state
                .get_pending_devices()
                .iter()
                .map(|d| {
                    d.get_name()
                        .clone()
                        .unwrap_or("<unknwon name>".to_string())
                        .clone()
                })
                .collect()
        });

        let devices_list = List::new(devices_list)
            .block(Block::default().title(Span::styled("Pending Devices", Style::new().bold())))
            .highlight_style(Style::new().bg(Color::DarkGray));

        let mut devices_list_state =
            ListState::default().with_selected(self.app.pending_state.device_selected());

        StatefulWidget::render(devices_list, chunks[0], buf, &mut devices_list_state);

        // Folders
        let folders_list: Vec<_> = self.app.state.read(|state| {
            state
                .get_pending_folders()
                .iter()
                .map(|(device_id, folder)| {
                    let device_name = match state.get_device(device_id) {
                        Ok(d) => &d.config.name,
                        Err(_) => "<unknown device>",
                    };

                    let text = match state.get_folder(folder.get_id()) {
                        Ok(_) => "[Share]",
                        Err(_) => "[Add]",
                    };
                    let label = folder
                        .get_label()
                        .clone()
                        .unwrap_or("<unknwon folder>".to_string());
                    Line::from(format!(
                        "{} \"{}\" ({}) - {}",
                        text,
                        label,
                        folder.get_id(),
                        device_name
                    ))
                })
                .collect()
        });

        let folders_list = List::new(folders_list)
            .block(Block::default().title(Span::styled("Pending Folders", Style::new().bold())))
            .highlight_style(Style::new().bg(Color::DarkGray));

        let mut folders_list_state =
            ListState::default().with_selected(self.app.pending_state.folder_selected());

        StatefulWidget::render(folders_list, chunks[1], buf, &mut folders_list_state);
    }
}
