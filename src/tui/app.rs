use std::sync::{Arc, Mutex};

use strum::IntoEnumIterator;
use tokio::sync::mpsc::UnboundedSender;

use crate::{AppError, Client};

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
    pub folders: Arc<Mutex<Vec<state::Folder>>>,
    pub selected_folder: Option<usize>,
    pub highlighted_folder: Option<usize>,
    pub error: Arc<Mutex<Option<AppError>>>,
}

impl App {
    pub fn new(client: Client, reload_tx: UnboundedSender<()>) -> Self {
        let app = App {
            client,
            reload_tx,
            running: true,
            current_screen: CurrentScreen::default(),
            folders: Arc::new(Mutex::new(vec![])),
            selected_folder: None,
            highlighted_folder: None,
            error: Arc::new(Mutex::new(None)),
        };
        app.load_folders();
        app
    }

    fn load_folders(&self) {
        let reload_tx = self.reload_tx.clone();
        let folders_handle = self.folders.clone();
        let error_handle = self.error.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let config = client.get_configuration().await;
            match config {
                Ok(conf) => {
                    *folders_handle.lock().unwrap() = conf.into();
                }
                Err(e) => *error_handle.lock().unwrap() = Some(e),
            }

            reload_tx.send(()).unwrap();
        });
    }

    fn update_folders(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::Down => {
                if let Some(highlighted_folder) = self.highlighted_folder {
                    self.highlighted_folder =
                        Some((highlighted_folder + 1) % self.folders.lock().unwrap().len())
                } else {
                    self.highlighted_folder = Some(0);
                }
            }
            Message::Up => {
                let len = self.folders.lock().unwrap().len();
                if let Some(highlighted_folder) = self.highlighted_folder {
                    self.highlighted_folder = Some((highlighted_folder + len - 1) % len)
                } else {
                    self.highlighted_folder = Some(len - 1);
                }
            }
            Message::Reload => {
                self.load_folders();
            }
            _ => {}
        };
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

mod state {
    use crate::Configuration;

    #[derive(Debug, PartialEq)]
    pub struct Folder {
        id: String,
        pub label: String,
        path: String, // or PathBuf?
        devices: Vec<Device>,
    }

    impl From<crate::ty::Folder> for Folder {
        fn from(folder: crate::ty::Folder) -> Self {
            Self {
                id: folder.id,
                label: folder.label,
                path: folder.path,
                devices: vec![], // TODO
            }
        }
    }

    impl From<Configuration> for Vec<Folder> {
        fn from(conf: Configuration) -> Self {
            let mut res = vec![];
            for folder in conf.folders {
                res.push(folder.into());
            }
            res
        }
    }

    #[derive(Debug, PartialEq)]
    pub struct Device {
        id: String,
        name: String,
    }
}
