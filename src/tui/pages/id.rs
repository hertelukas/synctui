use qrcode::QrCode;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    text::Text,
    widgets::Widget,
};
use tui_qrcode::QrCodeWidget;

pub struct IDPage {
    id: String,
}

impl IDPage {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

impl Widget for IDPage {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // TODO do error handling - e.g., just don't show QR code
        let qr_code = QrCode::new(&self.id).expect("could not generate QR code");
        let widget = QrCodeWidget::new(qr_code);

        let mut qr_area = center(
            area,
            Constraint::Length(widget.size(area).width),
            Constraint::Length(widget.size(area).height),
        );
        qr_area.y -= 1;

        let text = Text::raw(self.id);
        let [mut text_area] = Layout::horizontal([Constraint::Length(text.width() as u16)])
            .flex(Flex::Center)
            .areas(area);
        text_area.y = qr_area.y + qr_area.height;
        widget.render(qr_area, buf);
        text.render(text_area, buf);
    }
}
