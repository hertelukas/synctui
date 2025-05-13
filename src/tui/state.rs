use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use color_eyre::eyre;
use syncthing_rs::Client;
use syncthing_rs::types as api;
use syncthing_rs::types::config::NewDeviceConfiguration;
use syncthing_rs::types::config::NewFolderConfiguration;
use syncthing_rs::types::events::EventType;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

use crate::AppError;

#[derive(Copy, Clone, Debug)]
pub enum Reload {
    ID,
    Configuration,
    PendingDevices,
    PendingFolders,
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
            if let Err(e) = reload_tx.send(reload).await {
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
                            state.write(|state| state.update_from_configuration(conf));
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
                EventType::ConfigSaved {} => {
                    if let Err(e) = state.reload_tx.send(Reload::Configuration).await {
                        log::error!(
                            "failed to initiate configuration reload due to new saved config: {:?}",
                            e
                        );
                        state.set_error(e.into());
                    }
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
                _ => {}
            }
        }
    }

    /// Accept device `device_id` in the background. This function is
    /// non-blocking, and will emit a config update once the changes have
    /// been applied.
    ///
    /// # Errors
    ///
    /// Returns `UnknownDevice` if no such device exists as pending device.
    pub fn accept_device(&self, device_id: &str) -> Result<(), AppError> {
        let device = self.read(|state| state.get_pending_device(device_id).cloned())?;
        let state = self.clone();
        let self_handle = self.clone();
        tokio::spawn(async move {
            if let Err(e) = state.client.add_device(device).await {
                log::error!("failed to add to api: {:?}", e);
                state.set_error(e.into());
            } else {
                self_handle.reload(Reload::Configuration);
                // Don't care if updating subscriber fails
                let _ = state.config_tx.send(());
            }
        });
        Ok(())
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

        res.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        res
    }

    /// Get a configured device with id `device_id`
    pub fn get_device(&self, device_id: &str) -> eyre::Result<&Device, AppError> {
        self.devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or(AppError::UnknownDevice)
    }

    /// All devices, excluding the local device
    pub fn get_other_devices(&self) -> Vec<&Device> {
        self.get_devices()
            .into_iter()
            .filter(|device| device.id != self.id)
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
            .find(|f| f.id == folder_id)
            .ok_or(AppError::UnknownFolder)?;

        Ok(self
            .get_other_devices()
            .iter()
            .filter(|device| folder.shared_with.contains_key(&device.id))
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
        res.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
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
            .find(|f| f.id == folder_id)
            .ok_or(AppError::UnknownFolder)
    }

    pub fn get_folder_mut(&mut self, folder_id: &str) -> eyre::Result<&mut Folder, AppError> {
        self.folders
            .iter_mut()
            .find(|f| f.id == folder_id)
            .ok_or(AppError::UnknownFolder)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Folder {
    pub id: String,
    pub label: String,
    pub path: String, // or PathBuf?
    /// Whether the folder is in our configuration or only on a remote device
    pub shared_with: HashMap<String, FolderDeviceSharingDetails>,
}

impl Folder {
    pub fn new(id: String, label: String, path: String, devices: Vec<String>) -> Self {
        let mut hm = HashMap::new();
        for d in devices {
            hm.insert(d, FolderDeviceSharingDetails::new_configured());
        }
        Self {
            id,
            label,
            path,
            shared_with: hm,
        }
    }

    /// Get all the devices with which this folder is shared, sorted by device id
    pub fn get_sharer(&self) -> Vec<(&String, &FolderDeviceSharingDetails)> {
        let mut to_sort: Vec<_> = self.shared_with.iter().collect();
        to_sort.sort_by(|(a, _), (b, _)| a.cmp(b));
        to_sort
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FolderDeviceSharingDetails {
    /// Whether this folder is shared with that device
    pub remote_label: Option<String>,
}

impl FolderDeviceSharingDetails {
    pub fn new_configured() -> Self {
        Self { remote_label: None }
    }

    pub fn new_pending(label: Option<String>) -> Self {
        Self {
            remote_label: label,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub id: String,
    pub name: String,
}

impl Device {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name }
    }
}

impl From<api::config::DeviceConfiguration> for Device {
    fn from(value: api::config::DeviceConfiguration) -> Self {
        Self {
            id: value.device_id,
            name: value.name,
        }
    }
}

impl From<api::config::FolderConfiguration> for Folder {
    fn from(value: api::config::FolderConfiguration) -> Self {
        let mut hm = HashMap::new();
        for d in value.devices {
            hm.insert(d.device_id, FolderDeviceSharingDetails::new_configured());
        }
        Self {
            id: value.id,
            label: value.label,
            path: value.path,
            shared_with: hm,
        }
    }
}

impl From<api::events::AddedPendingFolderChanged> for Folder {
    fn from(value: api::events::AddedPendingFolderChanged) -> Self {
        let mut hm = HashMap::new();
        hm.insert(
            value.device_id,
            FolderDeviceSharingDetails::new_pending(Some(value.folder_label)),
        );
        Self {
            id: value.folder_id,
            label: String::new(),
            path: String::new(),
            shared_with: hm,
        }
    }
}

impl From<Folder> for api::config::FolderConfiguration {
    fn from(_value: Folder) -> Self {
        todo!(
            "this function should never be needed, just update
single fields"
        )
    }
}
