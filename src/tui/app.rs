use std::sync::{Arc, Mutex};

use state::State;
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

/// VIM modes
#[derive(Debug, Clone, PartialEq)]
pub enum CurrentMode {
    Insert,
    Normal,
}

impl std::fmt::Display for CurrentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Insert => write!(f, "I"),
            Self::Normal => write!(f, "N"),
        }
    }
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
    pub state: Arc<Mutex<Option<State>>>,
    pub selected_folder: Option<usize>,
    pub error: Arc<Mutex<Option<AppError>>>,
    pub mode: Arc<Mutex<CurrentMode>>,
}

impl App {
    pub fn new(client: Client, reload_tx: UnboundedSender<()>) -> Self {
        let app = App {
            client,
            reload_tx,
            running: true,
            current_screen: CurrentScreen::default(),
            state: Arc::new(Mutex::new(None)),
            selected_folder: None,
            error: Arc::new(Mutex::new(None)),
            mode: Arc::new(Mutex::new(CurrentMode::Normal)),
        };
        app.load_folders();
        app
    }

    fn load_folders(&self) {
        let reload_tx = self.reload_tx.clone();
        let state_handle = self.state.clone();
        let error_handle = self.error.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let config = client.get_configuration().await;
            match config {
                Ok(conf) => {
                    *state_handle.lock().unwrap() = Some(conf.into());
                }
                Err(e) => *error_handle.lock().unwrap() = Some(e),
            }

            reload_tx.send(()).unwrap();
        });
    }

    fn update_folders(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::Down => {
                if let Some(highlighted_folder) = self.selected_folder {
                    self.selected_folder = Some(
                        (highlighted_folder + 1)
                            % self
                                .state
                                .lock()
                                .unwrap()
                                .as_ref()
                                .map_or(0, |state| state.folders.len()),
                    )
                } else {
                    self.selected_folder = Some(0);
                }
            }
            Message::Up => {
                let len = self
                    .state
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map_or(0, |state| state.folders.len());
                if let Some(highlighted_folder) = self.selected_folder {
                    self.selected_folder = Some((highlighted_folder + len - 1) % len)
                } else {
                    self.selected_folder = Some(len - 1);
                }
            }
            Message::Reload => {
                self.load_folders();
            }
            Message::Select => {}
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
            Message::Insert => {
                *self.mode.lock().unwrap() = CurrentMode::Insert;
            }
            Message::Normal => {
                *self.mode.lock().unwrap() = CurrentMode::Normal;
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
    use std::collections::HashMap;

    use crate::Configuration;

    #[derive(Debug)]
    pub struct State {
        pub folders: Vec<Folder>,
        pub devices: HashMap<String, Device>,
    }

    #[derive(Debug, PartialEq)]
    pub struct Folder {
        pub id: String,
        pub label: String,
        pub path: String, // or PathBuf?
        device_ids: Vec<String>,
    }

    impl Folder {
        pub fn get_devices<'a>(&self, state: &'a State) -> Vec<&'a Device> {
            self.device_ids
                .iter()
                .filter_map(|id| state.devices.get(id))
                .collect()
        }
    }

    impl From<crate::ty::Folder> for Folder {
        fn from(folder: crate::ty::Folder) -> Self {
            let mut device_ids = vec![];
            for device in folder.devices {
                device_ids.push(device.device_id);
            }
            Self {
                id: folder.id,
                label: folder.label,
                path: folder.path,
                device_ids,
            }
        }
    }

    impl From<Configuration> for State {
        fn from(conf: Configuration) -> Self {
            let mut folders = vec![];
            let mut devices: HashMap<String, Device> = HashMap::new();
            for device in conf.devices {
                devices.insert(device.device_id.clone(), device.into());
            }
            for folder in conf.folders {
                folders.push(folder.into());
            }
            Self { folders, devices }
        }
    }

    #[derive(Debug, PartialEq)]
    pub struct Device {
        id: String,
        pub name: String,
    }

    impl From<crate::ty::Device> for Device {
        fn from(value: crate::ty::Device) -> Self {
            Self {
                id: value.device_id,
                name: value.name,
            }
        }
    }
}
