use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};
use strum::IntoEnumIterator;

use super::app::{App, CurrentScreen};

pub fn ui(frame: &mut Frame, app: &App) {
    frame.render_widget(create_background(app), frame.area());
    match app.current_screen {
        CurrentScreen::Folders => {
            frame.render_widget(folders_block(app), frame.area());
        }
        _ => {}
    };
}

fn folders_block(app: &App) -> impl Widget {
    let mut list_items = Vec::<ListItem>::new();

    //if let Ok(configuration) = app.client.get_configuration().await {
    //     for folder in configuration.folders {
    //         list_items.push(ListItem::new(Line::from(Span::raw(folder.label))));
    //     }
    // }

    let list = List::new(list_items);
    list.block(Block::bordered().borders(Borders::ALL))
}

fn create_background(app: &App) -> Block {
    let block = Block::default()
        .title_top(Line::from("| SyncTUI |").centered())
        .borders(Borders::ALL);

    let mut bottom_string = CurrentScreen::iter()
        .enumerate()
        .map(|(i, screen)| {
            Span::styled(
                format!("| ({}) {:?} ", i + 1, screen),
                if screen == app.current_screen {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            )
        })
        .collect::<Vec<Span>>();
    bottom_string.push("|".into());

    block
        .title_bottom(bottom_string)
        .title_bottom(Line::from("| (q) quit |").right_aligned())
}
