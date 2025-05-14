use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use strum::IntoEnumIterator;

use super::{
    app::{App, CurrentScreen},
    pages::{DevicesPage, FoldersPage, IDPage, PendingPage},
};

pub fn ui(frame: &mut Frame, app: &App) {
    // If we have an error, show only that
    if app.state.read(|state| {
        if let Some(error) = &state.error {
            let popup_block =
                create_popup_block(app, "Error".to_string()).style(Style::default().fg(Color::Red));

            let error_text = Text::styled(error.to_string(), Style::default().fg(Color::default()));
            let error_paragraph = Paragraph::new(error_text)
                .block(popup_block)
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(Wrap { trim: false }); // Do not cut off whn over edge

            let area = centered_rect(50, 50, frame.area());
            frame.render_widget(error_paragraph, area);
            true
        } else {
            false
        }
    }) {
        return;
    }

    let background = create_background(app);
    let inner_area = background.inner(frame.area());
    match app.current_screen {
        CurrentScreen::Folders => FoldersPage::new(app).render(inner_area, frame.buffer_mut()),
        CurrentScreen::Devices => DevicesPage::new(app).render(inner_area, frame.buffer_mut()),
        CurrentScreen::ID => IDPage::new(app.state.read(|state| state.id.clone()))
            .render(inner_area, frame.buffer_mut()),
        CurrentScreen::Pending => PendingPage::new(app).render(inner_area, frame.buffer_mut()),
    };

    frame.render_widget(background, frame.area());

    if let Some(popup) = &app.popup {
        let state = app.state.clone();
        popup.render(frame, state);
    }
}

fn create_background(app: &App) -> Block {
    let block = Block::default()
        .title_top(Line::from("| SyncTUI |").centered().bold())
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

    block.title_bottom(bottom_string).title_bottom(
        Line::from(format!("| (q) quit | {} |", app.mode.lock().unwrap())).right_aligned(),
    )
}

fn create_popup_block(_: &App, title: String) -> Block {
    let block = Block::default()
        .title_top(Line::from(format!("| {} |", title)).centered())
        .borders(Borders::ALL);

    block
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
