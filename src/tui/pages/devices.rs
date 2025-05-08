use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListState, StatefulWidget, Widget},
};

use crate::tui::app::App;

pub struct DevicesPage<'a> {
    app: &'a App,
}

impl<'a> DevicesPage<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl Widget for DevicesPage<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &DevicesPage<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let list: Vec<_> = self.app.state.read(|state| {
            state
                .get_other_devices()
                .iter()
                .map(|d| d.name.clone())
                .collect()
        });

        let list = List::new(list).highlight_style(Style::new().bg(Color::DarkGray));
        let mut list_state = ListState::default().with_selected(self.app.selected_device);

        StatefulWidget::render(list, chunks[0], buf, &mut list_state);

        if let Some(device_index) = self.app.selected_device {
            if let Some(device_name) = self.app.state.read(|state| {
                state
                    .get_devices()
                    .get(device_index)
                    .map(|d| d.name.clone())
            }) {
                let block = Block::default()
                    .title_top(Line::from(format!("| {} |", device_name)).centered())
                    .borders(Borders::ALL);
                block.render(chunks[1], buf);
            }
        }
    }
}
