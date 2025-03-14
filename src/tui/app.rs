use strum::IntoEnumIterator;
use tokio::sync::mpsc::UnboundedSender;

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
    pub client: Client,
    reload_tx: UnboundedSender<()>,
    pub running: bool,
    pub current_screen: CurrentScreen,
}

impl App {
    pub fn new(client: Client, reload_tx: UnboundedSender<()>) -> Self {
        App {
            client,
            reload_tx,
            running: true,
            current_screen: CurrentScreen::default(),
        }
    }

    fn update_folders(&mut self, msg: Message) -> Option<Message> {
        None
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {
        // First, handle global messages
        match msg {
            Message::Quit => {
                self.running = false;
                return None;
            }
            Message::Number(i) => {
                if let Ok(screen) = CurrentScreen::try_from(i) {
                    self.current_screen = screen;
                    return None;
                }
            }

            _ => {}
        }

        match self.current_screen {
            CurrentScreen::Folders => self.update_folders(msg),
            _ => None,
        }
    }
}
