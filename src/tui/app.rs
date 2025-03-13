use strum::IntoEnumIterator;

use crate::Client;

use super::input::Message;

#[derive(Default, Debug, strum::EnumIter, PartialEq)]
pub enum CurrentScreen {
    #[default]
    Folders,
    Devices,
}

impl TryFrom<u32> for CurrentScreen {
    type Error = ();

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        if let Some((_, screen)) = CurrentScreen::iter()
            .enumerate()
            .find(|(i, _)| i + 1 == (v as usize))
        {
            Ok(screen)
        } else {
            Err(())
        }
    }
}

/// Tracks current state of application
#[derive(Debug)]
pub struct App {
    client: Client,
    pub running: bool,
    pub current_screen: CurrentScreen,
}

impl App {
    pub fn new(client: Client) -> Self {
        App {
            client,
            running: true,
            current_screen: CurrentScreen::default(),
        }
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::Quit => self.running = false,
            Message::Number(i) => {
                if let Ok(screen) = CurrentScreen::try_from(i) {
                    self.current_screen = screen;
                }
            }

            _ => {}
        }
        None
    }
}
