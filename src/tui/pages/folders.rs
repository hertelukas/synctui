use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
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

        let list: Vec<_> = self.app.state.read(|state| {
            state
                .get_folders()
                .iter()
                .map(|f| (f.config.label.clone(), f.completion))
                .collect()
        });

        let max = list
            .iter()
            .max_by(|x, y| x.0.char_indices().count().cmp(&y.0.char_indices().count()))
            .map_or(0, |f| f.0.char_indices().count());

        let list: Vec<_> = list
            .iter()
            .map(|(label, completion)| {
                let online_span = if *completion == 100.0 {
                    Span::styled("[Up to Date]", Style::default().green().bold())
                } else {
                    Span::styled(format!("[{:.0}%]", completion), Style::default().red())
                };

                let spacing = (max + 2) - label.char_indices().count();
                Line::from(vec![
                    Span::raw(label),
                    Span::raw(" ".repeat(spacing)),
                    online_span,
                ])
            })
            .collect();

        let list = List::new(list).highlight_style(Style::new().bg(Color::DarkGray));

        let mut list_state = ListState::default().with_selected(self.app.selected_folder);

        StatefulWidget::render(list, chunks[0], buf, &mut list_state);

        if let Some(folder_index) = self.app.selected_folder {
            self.app.state.read(|state| {
                if let Some(folder) = state.get_folders().get(folder_index) {
                    let block = Block::default()
                        .title_top(
                            Line::from(format!("| {} |", folder.config.label))
                                .centered()
                                .bold(),
                        )
                        .borders(Borders::ALL);
                    // Folder information
                    let mut folder_info = Vec::<ListItem>::new();
                    folder_info.push(ListItem::new(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("ID", Style::default().bold()),
                        Span::raw(format!("          : {}", folder.config.id)),
                    ])));
                    folder_info.push(ListItem::new(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("Path", Style::default().bold()),
                        Span::raw(format!("        : {}", folder.config.path)),
                    ])));
                    folder_info.push(ListItem::new(Line::from("")));

                    let folder_sharer = folder.get_sharer_excluded(&state.id).len();
                    let s_suffix = if folder_sharer == 1 { "" } else { "s" };

                    folder_info.push(ListItem::new(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("Shared with", Style::default().bold()),
                        Span::raw(" : "),
                        Span::styled(format!("{}", folder_sharer), Style::default().bold()),
                        Span::raw(format!(" Device{}", s_suffix)),
                    ])));

                    for i in 0..folder_sharer {
                        if let Some(device_id) = folder.get_sharer_excluded(&state.id).get(i) {
                            let ident = if i < folder_sharer - 1 {
                                "├─"
                            } else {
                                "└─"
                            };
                            if let Ok(device) = state.get_device(device_id) {
                                folder_info.push(ListItem::new(Line::from(format!(
                                    "  {} {}",
                                    ident, device.config.name
                                ))));
                            }
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
