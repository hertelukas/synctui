use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListState, StatefulWidget, Widget},
};

use crate::tui::app::App;

pub struct PendingPage<'a> {
    app: &'a App,
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
        let devices_list: Vec<_> = self
            .app
            .state
            .lock()
            .unwrap()
            .pending_devices
            .get_sorted()
            .iter()
            .map(|(id, device)| Line::from(format!("{} ({})", device.name, id)))
            .collect();

        let devices_list = List::new(devices_list)
            .block(Block::default().title(Span::styled("Pending Devices", Style::new().bold())))
            .highlight_style(Style::new().bg(Color::DarkGray));

        let mut devices_list_state = ListState::default().with_selected(self.app.selected_pending);

        StatefulWidget::render(devices_list, chunks[0], buf, &mut devices_list_state);

        // Folders
        let folders_list: Vec<_> = {
            let state = self.app.state.lock().unwrap();
            state
                .pending_folders
                .get_sorted()
                .iter()
                .map(|(folder_id, folder)| {
                    folder
                        .offered_by
                        .iter()
                        .map(|(device_id, folder)| {
                            let device_name = if let Some(device) = state.devices.get(device_id) {
                                device.name.clone()
                            } else {
                                "Unknown device".to_string()
                            };
                            Line::from(format!(
                                "\"{}\" ({}) - {}",
                                folder.label, folder_id, device_name
                            ))
                        })
                        .collect::<Vec<_>>()
                })
                .flatten()
                .collect()
        };
        let folders_list = List::new(folders_list)
            .block(Block::default().title(Span::styled("Pending Folders", Style::new().bold())))
            .highlight_style(Style::new().bg(Color::DarkGray));

        let mut folders_list_state = ListState::default();

        StatefulWidget::render(folders_list, chunks[1], buf, &mut folders_list_state);
    }
}
