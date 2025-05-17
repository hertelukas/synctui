use std::sync::{Arc, Mutex};

use log::{debug, warn};
use strum::IntoEnumIterator;
use syncthing_rs::{
    Client,
    types::{
        config::NewFolderConfiguration,
        events::{Event, EventType},
    },
};
use tokio::sync::{broadcast, mpsc};

use crate::{AppError, tui::state::State};

use super::{
    input::Message,
    pages::PendingPageState,
    popup::{FolderPopup, NewFolderPopup, PendingDevicePopup, PendingShareFolderPopup, Popup},
    state::Reload,
};

#[derive(Default, Debug, strum::EnumIter, PartialEq)]
pub enum CurrentScreen {
    #[default]
    Folders,
    Devices,
    Pending,
    ID,
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
    rerender_tx: mpsc::Sender<Message>,
    pub running: bool,
    pub current_screen: CurrentScreen,
    pub state: State,
    pub selected_folder: Option<usize>,
    pub selected_device: Option<usize>,
    pub pending_state: PendingPageState,
    pub mode: Arc<Mutex<CurrentMode>>,
    pub popup: Option<Box<dyn Popup>>,
}

impl App {
    pub fn new(client: Client, rerender_tx: mpsc::Sender<Message>) -> Self {
        let app = App {
            rerender_tx,
            running: true,
            current_screen: CurrentScreen::default(),
            state: State::new(client.clone()),
            selected_folder: None,
            selected_device: None,
            pending_state: PendingPageState::default(),
            mode: Arc::new(Mutex::new(CurrentMode::Normal)),
            popup: None,
        };

        // React to events
        let rerender_tx = app.rerender_tx.clone();
        let event_rx = app.state.subscribe_to_events();
        tokio::spawn(async move { Self::handle_event(event_rx, rerender_tx).await });

        // Start listen to changes to the config and rerender based on them
        let rerender_tx = app.rerender_tx.clone();
        let config_rx = app.state.subscribe_to_config();
        tokio::spawn(async move { Self::handle_rerender(config_rx, rerender_tx).await });

        // TODO maybe reload state here again, as the state might already have fully
        // been fully initialized while we were setting up the listeners

        app
    }

    /// Runs in the background and reacts to Syncthing events.
    async fn handle_event(
        mut event_rx: broadcast::Receiver<Event>,
        rerender_tx: mpsc::Sender<Message>,
    ) {
        while let Ok(event) = event_rx.recv().await {
            debug!("Received event: {:?}", event);
            match event.ty {
                EventType::PendingDevicesChanged {
                    ref added,
                    ref removed,
                } => {
                    if let Some(added) = added {
                        if let Some(first) = added.first() {
                            if let Err(e) = rerender_tx
                                .send(Message::NewPendingDevice(first.device_id.clone()))
                                .await
                            {
                                warn!(
                                    "failed to send rerender message with new popup about new pending device: {:?}",
                                    e
                                );
                                // Don't set an error, as this is not really mission critical
                            }
                        }
                    }
                    if let Some(_removed) = removed {
                        // TODO close popup if we have one with a removed device opened
                    }
                }
                EventType::PendingFoldersChanged {
                    ref added,
                    ref removed,
                } => {
                    if let Some(added) = added {
                        if let Some(first) = added.first() {
                            if let Err(e) = rerender_tx
                                .send(Message::NewPendingFolder {
                                    folder_label: first.folder_label.clone(),
                                    folder_id: first.folder_id.clone(),
                                    device_id: first.device_id.clone(),
                                })
                                .await
                            {
                                warn!(
                                    "failed to send rerender message with new popup about new pending folder-share: {:?}",
                                    e
                                );
                            }
                        }
                    }
                    if let Some(_removed) = removed {
                        // TODO close popup if we have one with a removed folder opened
                    }
                }
                _ => {}
            }
        }
    }

    /// Listens to config changes and just initiates a rerender of the UI
    async fn handle_rerender(
        mut reload_rx: broadcast::Receiver<()>,
        rerender_tx: mpsc::Sender<Message>,
    ) {
        while reload_rx.recv().await.is_ok() {
            rerender_tx.send(Message::None).await.unwrap();
        }
        unreachable!("the config sender should never have been dropped")
    }

    fn update_folders(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::Down => {
                let len = self.state.read(|state| state.get_folders().len());
                if len == 0 {
                    return None;
                }
                if let Some(highlighted_folder) = self.selected_folder {
                    self.selected_folder = Some((highlighted_folder + 1) % len)
                } else {
                    self.selected_folder = Some(0);
                }
            }
            Message::Up => {
                let len = self.state.read(|state| state.get_folders().len());
                if len == 0 {
                    return None;
                }

                if let Some(highlighted_folder) = self.selected_folder {
                    self.selected_folder = Some((highlighted_folder + len - 1) % len)
                } else {
                    self.selected_folder = Some(len - 1);
                }
            }
            Message::Add => {
                self.popup = Some(Box::new(NewFolderPopup::new(
                    self.mode.clone(),
                    self.state.clone(),
                )));
            }
            Message::Select => {
                if let Some(highlighted_folder) = self.selected_folder {
                    self.state.read(|state| {
                        if let Some(folder) = state.get_folders().get(highlighted_folder) {
                            self.popup = Some(Box::new(FolderPopup::new(
                                folder.config.clone(),
                                self.mode.clone(),
                            )))
                        }
                    })
                }
            }
            _ => {}
        };
        None
    }

    fn update_devices(&mut self, msg: Message) -> Option<Message> {
        let len = self.state.read(|state| state.get_other_devices().len());
        match msg {
            Message::Down => {
                if len == 0 {
                    return None;
                }

                if let Some(highlighted_device) = self.selected_device {
                    self.selected_device = Some((highlighted_device + 1) % len)
                } else {
                    self.selected_device = Some(0)
                }
            }
            Message::Up => {
                if len == 0 {
                    return None;
                }
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

    fn update_pending(&mut self, msg: Message) -> Option<Message> {
        let devices_len = self.state.read(|state| state.get_pending_devices().len());

        let folders_len = self.state.read(|state| state.get_pending_folders().len());

        self.pending_state.update(&msg, devices_len, folders_len);
        if matches!(msg, Message::Select) {
            // Device Popup
            if let Some(index) = self.pending_state.device_selected() {
                self.state.read(|state| {
                    if let Some(device) = state.get_pending_devices().get(index) {
                        self.popup = Some(Box::new(PendingDevicePopup::new(
                            device.get_device_id().clone(),
                        )))
                    }
                });
            };
            // Folder Popup
            if let Some(index) = self.pending_state.folder_selected() {
                self.state.read(|state| {
                    if let Some((device_id, folder)) = state.get_pending_folders().get(index) {
                        // Only need to share, folder exists already locally
                        if state.get_folder(folder.get_id()).is_ok() {
                            self.popup = Some(Box::new(PendingShareFolderPopup::new(
                                folder.get_id().to_string(),
                                device_id.to_string(),
                            )))
                        } else {
                            self.popup = Some(Box::new(NewFolderPopup::new_from_device(
                                folder.get_label().clone().unwrap_or("".to_string()),
                                folder.get_id().to_string(),
                                device_id.to_string(),
                                self.mode.clone(),
                                self.state.clone(),
                            )))
                        }
                    }
                });
            }
        };
        None
    }

    fn handle_new_folder(&mut self, folder: NewFolderConfiguration) -> Option<Message> {
        // Raise an error if we have a duplicate id.
        // Probably, this should also be done in the state
        if self
            .state
            .read(|state| state.get_folder(folder.get_id()).is_ok())
        {
            self.state.set_error(AppError::DuplicateFolderID);
            return None;
        }

        // TODO maybe check that path is valid
        self.state.add_foler(folder);
        None
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {
        // Mode switches and popup results take always priority
        match msg {
            Message::Insert => *self.mode.lock().unwrap() = CurrentMode::Insert,
            Message::Normal => *self.mode.lock().unwrap() = CurrentMode::Normal,
            Message::NewFolder(folder) => {
                self.popup = None;
                return self.handle_new_folder(*folder);
            }
            Message::AcceptDevice(ref device) => {
                self.popup = None;
                self.state.accept_device(device);
            }
            Message::IgnoreDevice(_) => {
                self.popup = None;
                todo!("add device to ignore list");
            }
            Message::DismissDevice(ref device_id) => {
                self.popup = None;
                self.state.dismiss_device(device_id);
            }
            Message::ShareFolder {
                ref folder_id,
                ref device_id,
            } => {
                self.popup = None;
                self.state.share_folder(folder_id, device_id);
            }
            Message::DismissFolder {
                ref folder_id,
                ref device_id,
            } => {
                self.popup = None;
                self.state.dismiss_folder(folder_id, device_id);
            }
            _ => {}
        }

        // Then, we handle popups if one exists
        if let Some(popup) = self.popup.as_mut() {
            if let Some(msg) = popup.update(msg, self.state.clone()) {
                match msg {
                    Message::Quit => self.popup = None,
                    // All other messages from the popup are handles in the next
                    // iteration, normally. This allows for greater flexibility
                    _ => return Some(msg),
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
                self.state.reload(Reload::Configuration);
            }
            Message::NewPendingDevice(ref device) => {
                self.popup = Some(Box::new(PendingDevicePopup::new(device.clone())));
            }
            Message::NewPendingFolder {
                ref folder_label,
                ref folder_id,
                ref device_id,
            } => {
                // Folder already exists on our machine, just share
                if self.state.read(|state| state.get_folder(folder_id).is_ok()) {
                    self.popup = Some(Box::new(PendingShareFolderPopup::new(
                        folder_id.clone(),
                        device_id.to_string(),
                    )))
                } else {
                    self.popup = Some(Box::new(NewFolderPopup::new_from_device(
                        folder_label,
                        folder_id,
                        device_id,
                        self.mode.clone(),
                        self.state.clone(),
                    )))
                }
            }
            _ => {}
        }

        // Handle screen specific keys
        match self.current_screen {
            CurrentScreen::Folders => self.update_folders(msg),
            CurrentScreen::Devices => self.update_devices(msg),
            CurrentScreen::Pending => self.update_pending(msg),
            _ => None,
        }
    }
}
