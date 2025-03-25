use qrcode::QrCode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use strum::IntoEnumIterator;
use tui_qrcode::QrCodeWidget;

use super::app::{App, CurrentScreen};

pub fn ui(frame: &mut Frame, app: &App) {
    // If we have an error, show only that
    if let Some(error) = &*app.error.lock().unwrap() {
        let popup_block =
            create_popup_block(app, "Error".to_string()).style(Style::default().fg(Color::Red));

        let error_text = Text::styled(error.to_string(), Style::default().fg(Color::default()));
        let error_paragraph = Paragraph::new(error_text)
            .block(popup_block)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: false }); // Do not cut off whn over edge

        let area = centered_rect(50, 50, frame.area());
        frame.render_widget(error_paragraph, area);

        return;
    }

    let background = create_background(app);
    let inner_area = background.inner(frame.area());
    match app.current_screen {
        CurrentScreen::Folders => folders_block(frame, app, inner_area),
        CurrentScreen::Devices => devices_block(frame, app, inner_area),
        CurrentScreen::ID => qr_code_block(frame, app, inner_area),
    };

    frame.render_widget(background, frame.area());

    if let Some(popup) = &app.popup {
        popup.render(frame);
    }
}

/// Renders the folders page
fn folders_block(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let mut list_items = Vec::<ListItem>::new();

    for (i, folder) in app.state.lock().unwrap().folders.iter().enumerate() {
        list_items.push(ListItem::new(
            Line::from(Span::raw(folder.label.clone())).bg(app.selected_folder.map_or(
                Color::default(),
                |highlighted_folder| {
                    if highlighted_folder == i {
                        Color::DarkGray
                    } else {
                        Color::default()
                    }
                },
            )),
        ));
    }

    let list = List::new(list_items);

    frame.render_widget(list, chunks[0]);

    if let Some(folder_index) = app.selected_folder {
        let state = app.state.lock().unwrap();
        if let Some(folder) = state.folders.get(folder_index) {
            let block = Block::default()
                .title_top(Line::from(format!("| {} |", folder.label)).centered())
                .borders(Borders::ALL);
            // Folder information
            let mut folder_info = Vec::<ListItem>::new();
            folder_info.push(ListItem::new(Line::from(format!("ID: {}", folder.id))));
            folder_info.push(ListItem::new(Line::from(format!("Path: {}", folder.path))));
            folder_info.push(ListItem::new(Line::from(format!(
                "Shared with {} device{}",
                folder.get_devices(&state).len(),
                if folder.get_devices(&state).len() == 1 {
                    ""
                } else {
                    "s"
                }
            ))));
            for device in &folder.get_devices(&state) {
                folder_info.push(ListItem::new(Line::from(device.name.clone())));
            }
            let inner_area = block.inner(chunks[1]);
            frame.render_widget(block, chunks[1]);
            let list = List::new(folder_info);
            frame.render_widget(list, inner_area);
        }
    }
}

/// Renders the devices page
fn devices_block(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let mut devices_list = Vec::<ListItem>::new();

    for (i, device) in app
        .state
        .lock()
        .unwrap()
        .get_other_devices()
        .iter()
        .enumerate()
    {
        devices_list.push(ListItem::new(
            Line::from(Span::raw(device.name.clone())).bg(app.selected_device.map_or(
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

    frame.render_widget(list, chunks[0]);

    if let Some(device_index) = app.selected_device {
        if let Some(device) = app.state.lock().unwrap().get_devices().get(device_index) {
            let block = Block::default()
                .title_top(Line::from(format!("| {} |", device.name)).centered())
                .borders(Borders::ALL);
            frame.render_widget(block, chunks[1]);
        }
    }
}

fn qr_code_block(frame: &mut Frame, app: &App, area: Rect) {
    let qr_code =
        QrCode::new(app.state.lock().unwrap().id.clone()).expect("could not generate QR code");
    let widget = QrCodeWidget::new(qr_code);
    frame.render_widget(widget, area);
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
