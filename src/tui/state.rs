use std::sync::Arc;
use std::sync::RwLock;

use color_eyre::eyre;
use syncthing_rs::Client;
use syncthing_rs::types as api;
use syncthing_rs::types::config::DeviceConfiguration;
use syncthing_rs::types::config::FolderConfiguration;
use syncthing_rs::types::config::FolderDeviceConfiguration;
use syncthing_rs::types::config::NewDeviceConfiguration;
use syncthing_rs::types::config::NewFolderConfiguration;
use syncthing_rs::types::events::EventType;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

use crate::AppError;

#[derive(Clone, Debug)]
pub enum Reload {
    ID,
    Configuration,
    PendingDevices,
    PendingFolders,
    Connections,
    Completion {
        folder_id: Option<String>,
        device_id: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct State {
    client: Client,
    inner: Arc<RwLock<InnerState>>,
    event_tx: broadcast::Sender<api::events::Event>,
    config_tx: broadcast::Sender<()>,
    reload_tx: mpsc::Sender<Reload>,
}

impl State {
    pub fn new(client: Client) -> Self {
        let (event_tx, event_rx) = broadcast::channel(100);
        let (config_tx, _) = broadcast::channel(100);
        let (reload_tx, reload_rx) = mpsc::channel(10);
        let event_tx_clone = event_tx.clone();
        let client_clone = client.clone();

        let state = Self {
            client,
            inner: Arc::new(RwLock::new(InnerState::default())),
            event_tx,
            config_tx,
            reload_tx,
        };

        // Start listening to events
        let state_handle = state.clone();
        tokio::spawn(async move {
            if let Err(e) = client_clone.get_events(event_tx_clone, true).await {
                log::error!("failed to get events: {:?}", e);
                state_handle.set_error(e.into());
            };
        });

        // Start reacting to events
        let state_handle = state.clone();
        tokio::spawn(async move {
            Self::handle_event(event_rx, state_handle).await;
        });

        // Start listening to reloads
        let state_handle = state.clone();
        tokio::spawn(async move { Self::listen_to_reload(reload_rx, state_handle).await });

        // Start reloading everything ones.
        // These blocks all start a thread, so are non-blocking.
        state.reload(Reload::ID);
        state.reload(Reload::Configuration);
        state.reload(Reload::PendingDevices);
        state.reload(Reload::PendingFolders);

        state
    }

    pub fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&InnerState) -> R,
    {
        let guard = self.inner.read().unwrap();
        f(&guard)
    }

    /// Read only access to the state. External users should never
    /// have to modify the inner state directly, but should use functions
    /// in [`State`](Self)
    fn write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut InnerState) -> R,
    {
        let mut guard = self.inner.write().unwrap();
        f(&mut guard)
    }

    /// Initiate a reload of parts of the state, defined by `Reload`,
    /// by initiating a request to the API.
    pub fn reload(&self, reload: Reload) {
        let reload_tx = self.reload_tx.clone();
        let state = self.clone();
        tokio::spawn(async move {
            if let Err(e) = reload_tx.send(reload.clone()).await {
                log::error!("failed to initiate {:?} reload {:?}", reload, e);
                state.set_error(e.into());
            }
        });
    }

    pub fn set_error(&self, _error: AppError) {}

    pub fn clear_error(&self) {}

    /// Emits an [`Event`](api::events::Event) if a new one arrives
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<api::events::Event> {
        self.event_tx.subscribe()
    }

    /// Emits `()` if the config (everything except events) changes
    pub fn subscribe_to_config(&self) -> broadcast::Receiver<()> {
        self.config_tx.subscribe()
    }

    /// Starts listening to reload commands, and will start reloading parts
    /// of the configuration.
    // TODO maybe reload in separate threads, so reloads can be handled faster
    async fn listen_to_reload(mut reload_rx: mpsc::Receiver<Reload>, state: State) {
        while let Some(reload) = reload_rx.recv().await {
            match reload {
                Reload::Configuration => {
                    let config = state.client.get_configuration().await;
                    match config {
                        Ok(conf) => {
                            state.write(|state| state.update_from_configuration(conf.clone()));
                            state.reload(Reload::Connections);
                            for f in conf.folders {
                                state.reload(Reload::Completion {
                                    folder_id: Some(f.id),
                                    device_id: None,
                                });
                            }
                        }
                        Err(e) => {
                            log::error!("failed to reload config: {:?}", e);
                            state.set_error(e.into());
                        }
                    }
                }
                Reload::ID => {
                    let id = state.client.get_id().await;
                    match id {
                        Ok(id) => {
                            state.write(|state| state.id = id);
                        }
                        Err(e) => {
                            log::error!("failed to load Syncthing ID: {:?}", e);
                            state.set_error(e.into());
                        }
                    }
                }
                Reload::PendingDevices => {
                    let devices = state.client.get_pending_devices().await;
                    match devices {
                        Ok(devices) => state.write(|state| state.set_pending_devices(devices)),
                        Err(e) => log::warn!("failed to reload pending devices: {:?}", e),
                    }
                }
                Reload::PendingFolders => {
                    let folders = state.client.get_pending_folders().await;
                    match folders {
                        Ok(folders) => state.write(|state| state.set_pending_folders(folders)),
                        Err(e) => log::warn!("failed to reload pending folders: {:?}", e),
                    }
                }
                Reload::Connections => {
                    let connections = state.client.get_connections().await;
                    match connections {
                        Ok(connections) => state.write(|inner_state| {
                            for (device_id, connection) in connections.connections {
                                if let Ok(device) = inner_state.get_device_mut(&device_id) {
                                    if connection.connected {
                                        device.connected = DeviceStatus::UpToDate;
                                        state.reload(Reload::Completion {
                                            folder_id: None,
                                            device_id: Some(device_id),
                                        });
                                    } else {
                                        device.connected = DeviceStatus::Disconnected;
                                    }
                                }
                            }
                        }),
                        Err(e) => log::warn!("failed to reload connections: {:?}", e),
                    }
                }
                Reload::Completion {
                    folder_id,
                    device_id,
                } => {
                    let completion = state
                        .client
                        .get_completion(folder_id.as_deref(), device_id.as_deref())
                        .await;
                    match completion {
                        Ok(completion) => {
                            if let Some(device_id) = device_id {
                                if let Some(_folder_id) = folder_id {
                                    todo!("update folder completion for device");
                                } else {
                                    state.write(|state| {
                                        if let Ok(device) = state.get_device_mut(&device_id) {
                                            if completion.completion == 100.0 {
                                                device.connected = DeviceStatus::UpToDate
                                            } else {
                                                device.connected =
                                                    DeviceStatus::Syncing(completion.completion)
                                            }
                                        }
                                    })
                                }
                            }
                            // Set local completion of folder
                            else if let Some(folder_id) = folder_id {
                                state.write(|state| {
                                    if let Ok(folder) = state.get_folder_mut(&folder_id) {
                                        folder.completion = completion.completion;
                                    }
                                });
                            }
                        }
                        Err(e) => log::warn!("failed to reload completion: {:?}", e),
                    }
                }
            }
            // For every case, if we reach this point, the config has changed
            if let Err(e) = state.config_tx.send(()) {
                log::warn!(
                    "could not initiate a config update after a reload has been completed: {:?}",
                    e
                );
            }
        }
    }

    /// Some events motivate a reload of the configuration. That is done here
    /// in the background.
    async fn handle_event(mut event_rx: broadcast::Receiver<api::events::Event>, state: State) {
        while let Ok(event) = event_rx.recv().await {
            log::debug!("state is handling event {:?}", event);
            match event.ty {
                EventType::ConfigSaved { .. } => {
                    if let Err(e) = state.reload_tx.send(Reload::Configuration).await {
                        log::error!(
                            "failed to initiate configuration reload due to new saved config: {:?}",
                            e
                        );
                        state.set_error(e.into());
                    }
                }
                EventType::DeviceConnected { id, .. } => {
                    state.write(|state| {
                        log::debug!("Device {id} connected");
                        if let Ok(device) = state.get_device_mut(&id) {
                            device.connected = DeviceStatus::UpToDate;
                        }
                    });
                    // Not that important of an event
                    let _ = state.config_tx.send(());
                }
                EventType::DeviceDisconnected { id, .. } => {
                    state.write(|state| {
                        if let Ok(device) = state.get_device_mut(&id) {
                            device.connected = DeviceStatus::Disconnected;
                        }
                    });
                    // Not that important of an event
                    let _ = state.config_tx.send(());
                }
                EventType::PendingDevicesChanged { .. } => {
                    if let Err(e) = state.reload_tx.send(Reload::PendingDevices).await {
                        log::error!("failed to initiate pending devices reload: {:?}", e);
                        state.set_error(e.into());
                    }
                }
                EventType::PendingFoldersChanged { .. } => {
                    if let Err(e) = state.reload_tx.send(Reload::PendingFolders).await {
                        log::error!("failed to initiate pending devices reload: {:?}", e);
                        state.set_error(e.into());
                    }
                }
                EventType::RemoteDownloadProgress { ref device, .. } => {
                    if let Err(e) = state
                        .reload_tx
                        .send(Reload::Completion {
                            device_id: Some(device.to_string()),
                            folder_id: None,
                        })
                        .await
                    {
                        log::error!(
                            "failed to initiate completion status based on remote download progress: {:?}",
                            e
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Accept device `device_id` in the background. This function is
    /// non-blocking, and will emit a config update once the changes have
    /// been applied.
    pub fn accept_device(&self, device_id: &str) {
        match self.read(|state| state.get_pending_device(device_id).cloned()) {
            Ok(device) => {
                let state = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = state.client.add_device(device).await {
                        log::error!("failed to add device to api: {:?}", e);
                        state.set_error(e.into());
                    } else {
                        state.reload(Reload::Configuration);
                    }
                });
            }
            Err(e) => {
                log::error!("failed to accept device: {:?}", e);
                self.set_error(e);
            }
        }
    }

    /// Add a new folder
    pub fn add_foler(&self, folder: NewFolderConfiguration) {
        let state = self.clone();
        tokio::spawn(async move {
            if let Err(e) = state.client.add_folder(folder).await {
                log::error!("failed to add folder to api: {:?}", e);
                state.set_error(e.into());
            } else {
                // TODO We don't need to update the config, the event should handle that
                state.reload(Reload::Configuration);
            }
        });
    }

    pub fn share_folder(&self, folder_id: &str, device_id: &str) {
        if let Some(folder) = self.write(|state| match state.get_folder_mut(folder_id) {
            Ok(folder) => {
                folder.config.devices.push(FolderDeviceConfiguration {
                    device_id: device_id.to_string(),
                    introduced_by: String::new(),
                    encryption_password: String::new(),
                });
                Some(folder.config.clone())
            }
            Err(e) => {
                log::error!("fialed to share folder: {:?}", e);
                self.set_error(e);
                None
            }
        }) {
            let state = self.clone();
            tokio::spawn(async move {
                if let Err(e) = state.client.post_folder(folder).await {
                    log::error!("failed to share folder on api: {:?}", e);
                    state.set_error(e.into());
                }
            });
        }
    }

    pub fn edit_folder(&self, folder: FolderConfiguration) {
        let state = self.clone();
        tokio::spawn(async move {
            if let Err(e) = state.client.post_folder(folder).await {
                log::error!("failed to update folder on api: {:?}", e);
                state.set_error(e.into());
            }
        });
    }

    pub fn dismiss_folder(&self, folder_id: impl Into<String>, device_id: impl Into<String>) {
        let state = self.clone();
        let folder_id = folder_id.into();
        let device_id = device_id.into();
        tokio::spawn(async move {
            if let Err(e) = state
                .client
                .dismiss_pending_folder(&folder_id, Some(&device_id))
                .await
            {
                log::error!("failed to dismiss folder to api: {:?}", e);
                state.set_error(e.into());
            }
            // We don't need to update the config, the event should handle that
        });
    }

    pub fn remove_folder(&self, folder_id: impl Into<String>) {
        let state = self.clone();
        let folder_id = folder_id.into();

        tokio::spawn(async move {
            if let Err(e) = state.client.delete_folder(&folder_id).await {
                log::error!("failed to delete folder from api: {:?}", e);
                state.set_error(e.into());
            }
        });
    }

    pub fn dismiss_device(&self, device_id: impl Into<String>) {
        let state = self.clone();
        let device_id = device_id.into();
        tokio::spawn(async move {
            if let Err(e) = state.client.dismiss_pending_device(&device_id).await {
                log::error!("failed to dismiss device to api: {:?}", e);
                state.set_error(e.into());
            }
            // We don't need to update the config, the event should handle that
        });
    }
}

#[derive(Debug, Default)]
pub struct InnerState {
    folders: Vec<Folder>,
    devices: Vec<Device>,
    pending_folders: Vec<(String, NewFolderConfiguration)>,
    pending_devices: Vec<NewDeviceConfiguration>,
    pub events: Vec<api::events::Event>,
    pub error: Option<AppError>,
    /// The device ID of this device
    pub id: String,
}

impl InnerState {
    fn update_from_configuration(&mut self, configuration: api::config::Configuration) {
        self.folders.clear();
        self.devices.clear();
        for device in configuration.devices {
            self.devices.push(device.into());
        }
        for folder in configuration.folders {
            self.folders.push(folder.into());
        }
    }

    fn set_pending_devices(&mut self, pending_devices: api::cluster::PendingDevices) {
        self.pending_devices.clear();
        for (device_id, device) in pending_devices.devices.iter() {
            self.pending_devices
                .push(NewDeviceConfiguration::new(device_id.to_string()).name(device.name.clone()));
        }
    }

    fn set_pending_folders(&mut self, pending_folders: api::cluster::PendingFolders) {
        self.pending_folders.clear();
        for (folder_id, folder) in pending_folders.folders.iter() {
            for (introducer_id, offerer) in folder.offered_by.clone() {
                self.pending_folders.push((
                    introducer_id,
                    // TODO find a cleaner way to handle the unknown path at this point
                    NewFolderConfiguration::new(folder_id.to_string(), "?".to_string())
                        .label(offerer.label),
                ));
            }
        }

        log::debug!("Pending folders: {:#?}", self.get_pending_folders());
        log::debug!("Folders: {:#?}", self.get_folders());
    }

    /// All configured devices, sorted by name
    pub fn get_devices(&self) -> Vec<&Device> {
        let mut res: Vec<&Device> = self.devices.iter().collect();

        res.sort_by(|a, b| {
            a.config
                .name
                .to_lowercase()
                .cmp(&b.config.name.to_lowercase())
        });
        res
    }

    /// Get a configured device with id `device_id`
    pub fn get_device(&self, device_id: &str) -> eyre::Result<&Device, AppError> {
        self.devices
            .iter()
            .find(|d| d.config.device_id == device_id)
            .ok_or(AppError::UnknownDevice)
    }

    /// Get a configured device with id `device_id`
    pub fn get_device_mut(&mut self, device_id: &str) -> eyre::Result<&mut Device, AppError> {
        self.devices
            .iter_mut()
            .find(|d| d.config.device_id == device_id)
            .ok_or(AppError::UnknownDevice)
    }

    /// All devices, excluding the local device
    pub fn get_other_devices(&self) -> Vec<&Device> {
        self.get_devices()
            .into_iter()
            .filter(|device| device.config.device_id != self.id)
            .collect()
    }

    /// All devices with which `folder_id` is shared.
    pub fn get_devices_sharing_folder(
        &self,
        folder_id: &str,
    ) -> eyre::Result<Vec<&Device>, AppError> {
        let folder = self
            .folders
            .iter()
            .find(|f| f.config.id == folder_id)
            .ok_or(AppError::UnknownFolder)?;

        Ok(self
            .get_other_devices()
            .iter()
            .filter(|device| folder.get_sharer().contains(&&device.config.device_id))
            .copied()
            .collect())
    }

    /// All devices we have not yet configured
    pub fn get_pending_devices(&self) -> Vec<&NewDeviceConfiguration> {
        let mut res: Vec<&NewDeviceConfiguration> = self.pending_devices.iter().collect();

        // TODO lowercase
        res.sort_by(|a, b| a.get_name().cmp(b.get_name()));
        res
    }

    // Get device which has not yet been configured
    pub fn get_pending_device(
        &self,
        device_id: &str,
    ) -> eyre::Result<&NewDeviceConfiguration, AppError> {
        self.pending_devices
            .iter()
            .find(|d| d.get_device_id() == device_id)
            .ok_or(AppError::UnknownDevice)
    }

    /// All folders, sorted by name and then ID
    pub fn get_folders(&self) -> Vec<&Folder> {
        let mut res: Vec<&Folder> = self.folders.iter().collect();

        // TODO id
        res.sort_by(|a, b| {
            a.config
                .label
                .to_lowercase()
                .cmp(&b.config.label.to_lowercase())
        });
        res
    }

    pub fn get_pending_folders(&self) -> Vec<&(String, NewFolderConfiguration)> {
        let mut res: Vec<_> = self.pending_folders.iter().collect();

        // TODO lowercase & id
        // BUG this will return different orderings with respect to devices
        res.sort_by(|(_, a), (_, b)| a.get_label().cmp(b.get_label()));
        res
    }

    pub fn get_folder(&self, folder_id: &str) -> eyre::Result<&Folder, AppError> {
        self.folders
            .iter()
            .find(|f| f.config.id == folder_id)
            .ok_or(AppError::UnknownFolder)
    }

    pub fn get_folder_mut(&mut self, folder_id: &str) -> eyre::Result<&mut Folder, AppError> {
        self.folders
            .iter_mut()
            .find(|f| f.config.id == folder_id)
            .ok_or(AppError::UnknownFolder)
    }

    // Get all folders which are shared with `device_id`. Does not check
    // if `device_id` actually exists.
    pub fn get_device_folders(&self, device_id: &str) -> Vec<&Folder> {
        self.get_folders()
            .into_iter()
            .filter(|f| f.get_sharer().iter().any(|d| d == &device_id))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Folder {
    pub config: FolderConfiguration,
    pub completion: f64,
}

impl Folder {
    /// Get all the devices with which this folder is shared, sorted by device id
    pub fn get_sharer(&self) -> Vec<&String> {
        let mut to_sort: Vec<_> = self
            .config
            .devices
            .iter()
            .map(|folder_device_configuration| &folder_device_configuration.device_id)
            .collect();
        to_sort.sort();
        to_sort
    }

    /// Get all the devices with which this folder is shared, excluding `device_id`.
    /// This is especially useful for excluding the host.
    pub fn get_sharer_excluded(&self, device_id: &str) -> Vec<&String> {
        self.get_sharer()
            .into_iter()
            .filter(|d| d != &device_id)
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeviceStatus {
    UpToDate,
    Syncing(f64),
    Disconnected,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub config: DeviceConfiguration,
    pub connected: DeviceStatus,
}

impl From<api::config::DeviceConfiguration> for Device {
    fn from(value: api::config::DeviceConfiguration) -> Self {
        Self {
            config: value,
            connected: DeviceStatus::Disconnected,
        }
    }
}

impl From<api::config::FolderConfiguration> for Folder {
    fn from(folder: api::config::FolderConfiguration) -> Self {
        Self {
            config: folder,
            completion: 100.0,
        }
    }
}
