use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
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
            self.app.state.read(|state| {
                if let Some(device) = state.get_other_devices().get(device_index) {
                    let block = Block::default()
                        .title_top(Line::from(format!("| {} |", device.name)).centered().bold())
                        .borders(Borders::ALL);

                    // Device information
                    let mut device_info = Vec::<ListItem>::new();
                    device_info.push(ListItem::new(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("ID", Style::default().bold()),
                        Span::raw(format!("      : {}", device.id)),
                    ])));
                    device_info.push(ListItem::new(Line::from("")));

                    let device_folders = state.get_device_folders(&device.id).len();
                    let s_suffix = if device_folders == 1 { "" } else { "s" };

                    device_info.push(ListItem::new(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("Sharing", Style::default().bold()),
                        Span::raw(" : "),
                        Span::styled(format!("{}", device_folders), Style::default().bold()),
                        Span::raw(format!(" Folder{}", s_suffix)),
                    ])));

                    for i in 0..device_folders {
                        if let Some(folder) = state.get_device_folders(&device.id).get(i) {
                            let ident = if i < device_folders - 1 {
                                "├─"
                            } else {
                                "└─"
                            };
                            device_info.push(ListItem::new(Line::from(format!(
                                "  {} {}",
                                ident, folder.label
                            ))));
                        }
                    }

                    let inner_area = block.inner(chunks[1]);
                    block.render(chunks[1], buf);

                    let list = List::new(device_info);
                    Widget::render(list, inner_area, buf);
                }
            })
        }
    }
}
