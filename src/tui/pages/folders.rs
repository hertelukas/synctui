use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::tui::app::App;

pub struct FoldersPage<'a> {
    app: &'a App,
}

impl<'a> FoldersPage<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl Widget for FoldersPage<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &FoldersPage<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let list_items: Vec<_> = self.app.state.read(|state| {
            state
                .get_folders()
                .iter()
                .map(|f| f.label.clone())
                .collect()
        });

        let list = List::new(list_items).highlight_style(Style::new().bg(Color::DarkGray));

        let mut list_state = ListState::default().with_selected(self.app.selected_folder);

        StatefulWidget::render(list, chunks[0], buf, &mut list_state);

        if let Some(folder_index) = self.app.selected_folder {
            self.app.state.read(|state| {
                if let Some(folder) = state.get_folders().get(folder_index) {
                    let block = Block::default()
                        .title_top(Line::from(format!("| {} |", folder.label)).centered())
                        .borders(Borders::ALL);
                    // Folder information
                    let mut folder_info = Vec::<ListItem>::new();
                    folder_info.push(ListItem::new(Line::from(format!("ID: {}", folder.id))));
                    folder_info.push(ListItem::new(Line::from(format!("Path: {}", folder.path))));
                    folder_info.push(ListItem::new(Line::from(format!(
                        "Shared with {} device{}",
                        folder.get_sharer().len() - 1,
                        if folder.get_sharer().len() == 2 {
                            ""
                        } else {
                            "s"
                        }
                    ))));
                    for (device_id, _) in &folder.get_sharer() {
                        if &&state.id == device_id {
                            continue;
                        }
                        if let Ok(device) = state.get_device(device_id) {
                            folder_info.push(ListItem::new(Line::from(device.name.clone())));
                        }
                    }
                    let inner_area = block.inner(chunks[1]);
                    block.render(chunks[1], buf);
                    let list = List::new(folder_info);
                    Widget::render(list, inner_area, buf);
                }
            });
        }
    }
}
