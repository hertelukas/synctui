#[derive(Default, Debug, strum::EnumIter, PartialEq)]
pub enum CurrentScreen {
    #[default]
    Folders,
    Devices,
}

/// Tracks current state of application
#[derive(Default, Debug)]
pub struct App {
    pub current_screen: CurrentScreen,
}

impl App {
    pub fn set_screen(&mut self, screen: CurrentScreen) {
        self.current_screen = screen;
    }
}
