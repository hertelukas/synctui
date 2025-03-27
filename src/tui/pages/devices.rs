use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
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

        let mut devices_list = Vec::<ListItem>::new();

        for (i, device) in self
            .app
            .state
            .lock()
            .unwrap()
            .get_other_devices()
            .iter()
            .enumerate()
        {
            devices_list.push(ListItem::new(
                Line::from(Span::raw(device.name.clone())).bg(self.app.selected_device.map_or(
                    Color::default(),
                    |highlighted_device| {
                        if highlighted_device == i {
                            Color::DarkGray
                        } else {
                            Color::default()
                        }
                    },
                )),
            ));
        }

        let list = List::new(devices_list);
        list.render(chunks[0], buf);

        if let Some(device_index) = self.app.selected_device {
            if let Some(device) = self
                .app
                .state
                .lock()
                .unwrap()
                .get_devices()
                .get(device_index)
            {
                let block = Block::default()
                    .title_top(Line::from(format!("| {} |", device.name)).centered())
                    .borders(Borders::ALL);
                block.render(chunks[1], buf);
            }
        }
    }
}
