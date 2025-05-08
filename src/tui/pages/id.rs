use qrcode::QrCode;
use ratatui::widgets::Widget;
use tui_qrcode::QrCodeWidget;

pub struct IDPage {
    id: String,
}

impl IDPage {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

impl Widget for IDPage {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // TODO do error handling - e.g., just don't show QR code
        let qr_code = QrCode::new(self.id).expect("could not generate QR code");
        let widget = QrCodeWidget::new(qr_code);
        widget.render(area, buf);
    }
}
