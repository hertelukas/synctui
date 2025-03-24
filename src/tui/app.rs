use std::sync::{Arc, Mutex};

use state::State;
use strum::IntoEnumIterator;
use tokio::sync::mpsc::UnboundedSender;

use crate::{AppError, Client};

use super::{
    input::Message,
    popup::{NewFolderPopup, Popup},
};

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
    pub selected_device: Option<usize>,
    pub error: Arc<Mutex<Option<AppError>>>,
    pub mode: Arc<Mutex<CurrentMode>>,
    pub popup: Option<Box<dyn Popup>>,
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
            selected_device: None,
            error: Arc::new(Mutex::new(None)),
            mode: Arc::new(Mutex::new(CurrentMode::Normal)),
            popup: None,
        };
        app.reload_configuration();
        app
    }

    fn reload_configuration(&self) {
        let reload_tx = self.reload_tx.clone();
        let state_handle = self.state.clone();
        let error_handle = self.error.clone();
        let client = self.client.clone();
        // Spawn a thread which notifies our UI as soon as we get an API response
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
            Message::Add => {
                self.popup = Some(Box::new(NewFolderPopup::default()));
            }
            _ => {}
        };
        None
    }

    fn update_devices(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::Down => {
                if let Some(highlighted_device) = self.selected_device {
                    self.selected_device = Some(
                        (highlighted_device + 1)
                            % self
                                .state
                                .lock()
                                .unwrap()
                                .as_ref()
                                .map_or(0, |state| state.devices.len()),
                    )
                } else {
                    self.selected_device = Some(0)
                }
            }
            Message::Up => {
                let len = self
                    .state
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map_or(0, |state| state.devices.len());
                if let Some(highlighted_device) = self.selected_device {
                    self.selected_device = Some((highlighted_device + len - 1) % len)
                } else {
                    self.selected_device = Some(len - 1);
                }
            }
            _ => {}
        };
        None
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {
        // Mode switches take always priority
        match msg {
            Message::Insert => *self.mode.lock().unwrap() = CurrentMode::Insert,
            Message::Normal => *self.mode.lock().unwrap() = CurrentMode::Normal,
            _ => {}
        }

        // Then, we handle popups if one exists
        if let Some(popup) = self.popup.as_mut() {
            if let Some(msg) = popup.update(msg, self.state.clone()) {
                match msg {
                    Message::Quit => self.popup = None,
                    _ => {}
                }
            }
            return None;
        };

        // If there is none, handle global messages
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
            Message::Reload => {
                self.reload_configuration();
            }
            _ => {}
        }

        // Handle screen specific keys
        match self.current_screen {
            CurrentScreen::Folders => self.update_folders(msg),
            CurrentScreen::Devices => self.update_devices(msg),
        }
    }
}

pub mod state {
    use std::collections::HashMap;

    use crate::Configuration;

    #[derive(Debug)]
    pub struct State {
        pub folders: Vec<Folder>,
        /// Maps device_id to devices
        pub devices: HashMap<String, Device>,
    }

    impl State {
        pub fn get_devices(&self) -> Vec<&Device> {
            let mut res: Vec<&Device> = self.devices.values().collect();

            res.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            res
        }
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
