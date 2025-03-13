use crate::Client;

#[derive(Default, Debug, strum::EnumIter, PartialEq)]
pub enum CurrentScreen {
    #[default]
    Folders,
    Devices,
}

/// Tracks current state of application
#[derive(Debug)]
pub struct App {
    client: Client,
    pub current_screen: CurrentScreen,
}

impl App {
    pub fn new(client: Client) -> Self {
        App {
            client,
            current_screen: CurrentScreen::default(),
        }
    }
    pub fn set_screen(&mut self, screen: CurrentScreen) {
        self.current_screen = screen;
    }
}
