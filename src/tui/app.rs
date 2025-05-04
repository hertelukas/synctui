use std::sync::{Arc, Mutex};

use log::{debug, error, warn};
use strum::IntoEnumIterator;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{
    AppError,
    api::{Event, EventType, client::Client},
    tui::state::State,
};

use super::{
    input::Message,
    pages::PendingPageState,
    popup::{NewFolderPopup, PendingDevicePopup, PendingShareFolderPopup, Popup},
    state::{Folder, SharingState},
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
    // TODO remove this, the app should not directly need the client,
    // but go through the state
    client: Client,
    rerender_tx: Sender<Message>,
    reload_tx: Sender<Reload>,
    pub running: bool,
    pub current_screen: CurrentScreen,
    pub state: Arc<Mutex<State>>,
    pub selected_folder: Option<usize>,
    pub selected_device: Option<usize>,
    pub pending_state: PendingPageState,
    pub error: Arc<Mutex<Option<AppError>>>,
    pub mode: Arc<Mutex<CurrentMode>>,
    pub popup: Option<Box<dyn Popup>>,
}

#[derive(Copy, Clone, Debug)]
pub enum Reload {
    ID,
    Configuration,
    PendingDevices,
    PendingFolders,
}

impl App {
    pub fn new(client: Client, rerender_tx: Sender<Message>) -> Self {
        let (reload_tx, reload_rx) = mpsc::channel(10);
        let app = App {
            client: client.clone(),
            rerender_tx,
            reload_tx: reload_tx.clone(),
            running: true,
            current_screen: CurrentScreen::default(),
            state: Arc::new(Mutex::new(State::new(client.clone()))),
            selected_folder: None,
            selected_device: None,
            pending_state: PendingPageState::default(),
            error: Arc::new(Mutex::new(None)),
            mode: Arc::new(Mutex::new(CurrentMode::Normal)),
            popup: None,
        };

        // React to events
        let rerender_tx = app.rerender_tx.clone();
        let state_handle = app.state.clone();
        let error_handle = app.error.clone();
        let client_handle = client.clone();

        let (event_tx, event_rx) = mpsc::channel(10);

        // Start listening to events
        tokio::spawn(async move {
            if let Err(e) = client_handle.get_events(event_tx, true).await {
                error!("failed to get events: {:?}", e);
                *error_handle.lock().unwrap() = Some(e)
            };
        });

        let error_handle = app.error.clone();
        let reload_tx = reload_tx;
        // Update state.events if we get a new one
        tokio::spawn(async move {
            Self::handle_event(state_handle, event_rx, reload_tx, error_handle, rerender_tx).await
        });

        // Let everyone who ownes a reload_config_tx handle to update the config
        let rerender_tx = app.rerender_tx.clone();
        let state_handle = app.state.clone();
        let error_handle = app.error.clone();
        tokio::spawn(async move {
            Self::handle_reload(reload_rx, client, state_handle, rerender_tx, error_handle).await
        });

        app.reload(Reload::ID);
        app.reload(Reload::Configuration);
        app.reload(Reload::PendingDevices);
        app.reload(Reload::PendingFolders);

        app
    }

    /// Runs in the background and reacts to Syncthing events.
    async fn handle_event(
        state: Arc<Mutex<State>>,
        mut event_rx: Receiver<Event>,
        reload_tx: Sender<Reload>,
        error: Arc<Mutex<Option<AppError>>>,
        rerender_tx: Sender<Message>,
    ) {
        while let Some(event) = event_rx.recv().await {
            debug!("Received event: {:?}", event);
            match event.ty {
                EventType::ConfigSaved {} => {
                    if let Err(e) = reload_tx.send(Reload::Configuration).await {
                        error!(
                            "failed to initiate configuration reload due to new saved config: {:?}",
                            e
                        );
                        *error.lock().unwrap() = Some(e.into());
                    }
                }
                // TODO close popup if the pending device was removed
                EventType::PendingDevicesChanged {
                    ref added,
                    removed: _,
                } => {
                    if let Some(added) = added {
                        if let Some(first) = added.first() {
                            if let Err(e) = rerender_tx
                                .send(Message::NewPendingDevice(first.clone().into()))
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
                    if let Err(e) = reload_tx.send(Reload::PendingDevices).await {
                        error!("failed to initiate pending devices reload: {:?}", e);
                        *error.lock().unwrap() = Some(e.into());
                    }
                }
                EventType::PendingFoldersChanged {
                    ref added,
                    removed: _,
                } => {
                    if let Some(added) = added {
                        if let Some(first) = added.first() {
                            if let Err(e) = rerender_tx
                                .send(Message::NewPendingFolder(
                                    first.folder_id.clone(),
                                    first.device_id.clone(),
                                ))
                                .await
                            {
                                warn!(
                                    "failed to send rerender message with new popup about new pending folder-share: {:?}",
                                    e
                                );
                            }
                        }
                    }
                    if let Err(e) = reload_tx.send(Reload::PendingFolders).await {
                        error!("failed to initiate pending devices reload: {:?}", e);
                        *error.lock().unwrap() = Some(e.into());
                    }
                }
                _ => {}
            }
            state.lock().unwrap().events.push(event);
        }
    }

    /// Runs in the background and allows to initiate to asynchrounously start
    /// fetching data from the API and updating the current state.
    async fn handle_reload(
        mut reload_rx: Receiver<Reload>,
        client: Client,
        state: Arc<Mutex<State>>,
        rerender_tx: Sender<Message>,
        error: Arc<Mutex<Option<AppError>>>,
    ) {
        while let Some(reload) = reload_rx.recv().await {
            match reload {
                Reload::Configuration => {
                    let config = client.get_configuration().await;
                    match config {
                        Ok(conf) => {
                            state.lock().unwrap().update_from_configuration(conf);
                        }
                        Err(e) => {
                            error!("failed to reload config: {:?}", e);
                            *error.lock().unwrap() = Some(e);
                        }
                    }

                    rerender_tx.send(Message::None).await.unwrap();
                }
                Reload::ID => {
                    let id = client.get_id().await;
                    match id {
                        Ok(id) => {
                            state.lock().unwrap().id = id;
                        }
                        Err(e) => {
                            error!("failed to load Syncthing ID: {:?}", e);
                            *error.lock().unwrap() = Some(e);
                        }
                    }
                    rerender_tx.send(Message::None).await.unwrap();
                }
                Reload::PendingDevices => {
                    let devices = client.get_pending_devices().await;
                    match devices {
                        Ok(devices) => state.lock().unwrap().set_pending_devices(devices),
                        Err(e) => warn!("failed to reload pending devices: {:?}", e),
                    }
                    rerender_tx.send(Message::None).await.unwrap();
                }
                Reload::PendingFolders => {
                    let folders = client.get_pending_folders().await;
                    match folders {
                        Ok(folders) => state.lock().unwrap().set_pending_folders(folders),
                        Err(e) => warn!("failed to reload pending folders: {:?}", e),
                    }
                    rerender_tx.send(Message::None).await.unwrap();
                }
            }
        }
    }

    fn reload(&self, reload: Reload) {
        let reload_tx = self.reload_tx.clone();
        let error_handle = self.error.clone();
        tokio::spawn(async move {
            if let Err(e) = reload_tx.send(reload).await {
                error!("failed to initiate {:?} reload {:?}", reload, e);
                *error_handle.lock().unwrap() = Some(e.into());
            }
        });
    }

    fn update_folders(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::Down => {
                let len = self.state.lock().unwrap().get_folders().len();
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
                let len = self.state.lock().unwrap().get_folders().len();
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
            _ => {}
        };
        None
    }

    fn update_devices(&mut self, msg: Message) -> Option<Message> {
        let len = self.state.lock().unwrap().get_other_devices().len();
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
        let devices_len = self.state.lock().unwrap().get_pending_devices().len();

        let folders_len = self.state.lock().unwrap().get_pending_folder_sharer().len();

        self.pending_state.update(&msg, devices_len, folders_len);
        if matches!(msg, Message::Select) {
            // Device Popup
            if let Some(index) = self.pending_state.device_selected() {
                if let Some(device) = self.state.lock().unwrap().get_pending_devices().get(index) {
                    self.popup = Some(Box::new(PendingDevicePopup::new(device.id.clone())))
                };
            };
            // Folder Popup
            if let Some(index) = self.pending_state.folder_selected() {
                let state_handle = self.state.lock().unwrap();
                if let Some((folder, (device_id, _))) =
                    state_handle.get_pending_folder_sharer().get(index)
                {
                    // Only need to share, folder exists already locally
                    if folder.state == SharingState::Configured {
                        self.popup = Some(Box::new(PendingShareFolderPopup::new(
                            folder.id.clone(),
                            device_id.to_string(),
                        )))
                    } else {
                        unimplemented!("new (unknown) folder sharing");
                    }
                }
            }
        };
        None
    }

    fn handle_new_folder(&mut self, folder: Folder) -> Option<Message> {
        // Raise an error if we have a duplicate id
        if self
            .state
            .lock()
            .unwrap()
            .get_folders()
            .iter()
            .any(|f| f.id == folder.id)
        {
            *self.error.lock().unwrap() = Some(AppError::DuplicateFolderID);
            return None;
        }

        // TODO maybe check that path is valid

        let reload_tx = self.rerender_tx.clone();
        let client = self.client.clone();
        let error_handle = self.error.clone();
        tokio::spawn(async move {
            // TODO this should be done by the state, so we don't have
            // to care about updating the state etc.
            if let Err(e) = client.post_folder(folder.into()).await {
                error!("failed to post new folder: {:?}", e);
                *error_handle.lock().unwrap() = Some(e);
            }
            reload_tx.send(Message::None).await.unwrap();
        });
        None
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {
        // Mode switches and popup results take always priority
        match msg {
            Message::Insert => *self.mode.lock().unwrap() = CurrentMode::Insert,
            Message::Normal => *self.mode.lock().unwrap() = CurrentMode::Normal,
            Message::NewFolder(folder) => {
                self.popup = None;
                return self.handle_new_folder(folder);
            }
            Message::AcceptDevice(ref _device) => {
                self.popup = None;
                let _client = self.client.clone();
                let _error_handle = self.error.clone();
                tokio::spawn(async move {
                    // TODO do this in the state to handle updating correctly
                    todo!();
                    // if let Err(e) = client.add_device(device.into()).await {
                    //     error!("failed to add new device: {:?}", e);
                    //     *error_handle.lock().unwrap() = Some(e);
                    // }
                });
            }
            Message::IgnoreDevice(_) => {
                self.popup = None;
                todo!("add device to ignore list");
            }
            Message::DismissDevice(ref device) => {
                self.popup = None;
                let client = self.client.clone();
                let error_handle = self.error.clone();
                let device = device.to_string();
                tokio::spawn(async move {
                    if let Err(e) = client.delete_pending_device(&device).await {
                        error!("failed to delete pending device: {:?}", e);
                        *error_handle.lock().unwrap() = Some(e);
                    }
                });
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
                self.reload(Reload::Configuration);
            }
            Message::NewPendingDevice(ref device) => {
                self.popup = Some(Box::new(PendingDevicePopup::new(device.id.clone())));
            }
            Message::NewPendingFolder(ref folder_id, ref device_id) => {
                // Folder already exists on our machine, just share
                if self
                    .state
                    .lock()
                    .unwrap()
                    .get_folders()
                    .iter()
                    .any(|f| f.id == *folder_id)
                {
                    self.popup = Some(Box::new(PendingShareFolderPopup::new(
                        folder_id.clone(),
                        device_id.to_string(),
                    )))
                } else {
                    unimplemented!("handle new folder")
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
