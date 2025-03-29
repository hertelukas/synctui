use ratatui::{
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
        let list: Vec<_> = self
            .app
            .state
            .lock()
            .unwrap()
            .pending_devices
            .get_sorted()
            .iter()
            .map(|(id, device)| Line::from(format!("{} ({})", device.name, id)))
            .collect();

        let list = List::new(list)
            .block(Block::default().title(Span::styled("Pending Devices", Style::new().bold())))
            .highlight_style(Style::new().bg(Color::DarkGray));

        let mut devices_list_state = ListState::default().with_selected(self.app.selected_pending);

        StatefulWidget::render(list, area, buf, &mut devices_list_state);
    }
}
