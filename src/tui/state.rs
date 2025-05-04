use std::collections::HashMap;

use color_eyre::eyre;

use crate::api::types::{FolderDevice, XattrFilter};
use crate::api::{Event, types as api};
use crate::{AppError, api::client::Client};

#[derive(Debug)]
pub struct State {
    _client: Client,
    folders: Vec<Folder>,
    devices: Vec<Device>,
    pub events: Vec<Event>,
    /// Local Syncthing ID
    pub id: String,
}

impl State {
    pub fn new(client: Client) -> Self {
        Self {
            _client: client,
            folders: Vec::default(),
            devices: Vec::default(),
            events: Vec::default(),
            id: String::new(),
        }
    }

    pub fn update_from_configuration(&mut self, configuration: api::Configuration) {
        self.folders.clear();
        self.devices.clear();
        for device in configuration.devices {
            self.devices.push(device.into());
        }
        for folder in configuration.folders {
            self.folders.push(folder.into());
        }
    }

    pub fn set_pending_devices(&mut self, pending_devices: api::PendingDevices) {
        for (device_id, device) in pending_devices.devices.iter() {
            if let Ok(d) = self.get_device(device_id) {
                if d.state == SharingState::Configured {
                    log::warn!("pending device {:?} is already configured", d);
                }
            } else {
                log::debug!("adding new pending device {:?}", device);
                self.devices.push(Device::new_pending(
                    device_id.to_string(),
                    device.name.clone(),
                ));
            }
        }
    }

    pub fn set_pending_folders(&mut self, pending_folders: api::PendingFolders) {
        for (folder_id, folder) in pending_folders.folders.iter() {
            if let Ok(f) = self.get_folder_mut(folder_id) {
                // Check if we share with that device
                for (device_id, _) in folder.offered_by.iter() {
                    if let Some(sharing_details) = f.shared_with.get(device_id) {
                        if sharing_details.state == SharingState::Configured {
                            log::warn!("pending folder {:?} is already configured", folder);
                        }
                    } else {
                        log::debug!(
                            "new pending device {:?} on existing folder {:?}",
                            device_id,
                            folder
                        );
                        f.shared_with.insert(
                            device_id.clone(),
                            FolderDeviceSharingDetails::new_pending(None),
                        );
                    }
                }
            } else {
                self.folders.push(Folder::new_pending(
                    folder_id.to_string(),
                    folder
                        .offered_by
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.label.clone()))
                        .collect(),
                ));
            }
        }

        log::debug!("Pending folders: {:#?}", self.get_pending_folder_sharer());
        log::debug!("Folders: {:#?}", self.get_folders());
    }

    /// All devices, sorted by name
    pub fn get_devices(&self) -> Vec<&Device> {
        let mut res: Vec<&Device> = self.devices.iter().collect();

        res.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        res
    }

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
    pub fn get_pending_devices(&self) -> Vec<&Device> {
        self.get_devices()
            .into_iter()
            .filter(|device| device.state == SharingState::Pending)
            .collect()
    }

    /// All folders, sorted by name and then ID
    pub fn get_folders(&self) -> Vec<&Folder> {
        let mut res: Vec<&Folder> = self.folders.iter().collect();

        res.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
        res
    }

    pub fn get_pending_folders(&self) -> Vec<&Folder> {
        self.get_folders()
            .into_iter()
            .filter(|folder| folder.state == SharingState::Pending)
            .collect()
    }

    /// Returns all folders which have have someone who wants
    /// to share the folder with us. This means that a folder
    /// might appear multiple times, if multiple devices want
    /// to share it with us.
    pub fn get_pending_folder_sharer(
        &self,
    ) -> Vec<(&Folder, (&String, &FolderDeviceSharingDetails))> {
        self.get_folders()
            .into_iter()
            .flat_map(|f| {
                f.get_pending_sharer()
                    .into_iter()
                    .map(move |s| (f, s))
                    .collect::<Vec<_>>()
            })
            .collect()
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

#[derive(Debug, PartialEq, Eq)]
pub enum SharingState {
    Configured,
    Pending,
}

#[derive(Debug, PartialEq)]
pub struct Folder {
    pub id: String,
    pub label: String,
    pub path: String, // or PathBuf?
    /// Whether the folder is in our configuration or only on a remote device
    pub state: SharingState,
    pub shared_with: HashMap<String, FolderDeviceSharingDetails>,
}

impl Folder {
    pub fn new(id: String, label: String, path: String, devices: Vec<String>) -> Self {
        let mut hm = HashMap::new();
        for d in devices {
            hm.insert(d, FolderDeviceSharingDetails::new());
        }
        Self {
            id,
            label,
            path,
            state: SharingState::Configured,
            shared_with: hm,
        }
    }

    pub fn new_pending(id: String, devices: Vec<(String, String)>) -> Self {
        let mut hm = HashMap::new();
        for (device_id, remote_label) in devices {
            hm.insert(
                device_id,
                FolderDeviceSharingDetails::new_pending(Some(remote_label)),
            );
        }
        Self {
            id,
            label: String::new(),
            path: String::new(),
            state: SharingState::Pending,
            shared_with: hm,
        }
    }

    /// Get all the devices with which this folder is shared, sorted by device id
    pub fn get_sharer(&self) -> Vec<(&String, &FolderDeviceSharingDetails)> {
        let mut to_sort: Vec<_> = self.shared_with.iter().collect();
        to_sort.sort_by(|(a, _), (b, _)| a.cmp(b));
        to_sort
    }

    pub fn get_pending_sharer(&self) -> Vec<(&String, &FolderDeviceSharingDetails)> {
        self.get_sharer()
            .into_iter()
            .filter(|(_, details)| details.state == SharingState::Pending)
            .collect()
    }

    pub fn get_configured_sharer(&self) -> Vec<(&String, &FolderDeviceSharingDetails)> {
        self.get_sharer()
            .into_iter()
            .filter(|(_, details)| details.state == SharingState::Configured)
            .collect()
    }
}

#[derive(Debug, PartialEq)]
pub struct FolderDeviceSharingDetails {
    /// Whether this folder is shared with that device
    pub state: SharingState,
    pub remote_label: Option<String>,
}

impl FolderDeviceSharingDetails {
    pub fn new() -> Self {
        Self {
            state: SharingState::Configured,
            remote_label: None,
        }
    }

    pub fn new_pending(label: Option<String>) -> Self {
        Self {
            state: SharingState::Pending,
            remote_label: label,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub state: SharingState,
}

impl Device {
    pub fn new_pending(id: String, name: String) -> Self {
        Self {
            id,
            name,
            state: SharingState::Pending,
        }
    }
}

impl From<api::DeviceConfiguration> for Device {
    fn from(value: api::DeviceConfiguration) -> Self {
        Self {
            id: value.device_id,
            name: value.name,
            state: SharingState::Configured,
        }
    }
}

impl From<api::AddedPendingDevice> for Device {
    fn from(value: api::AddedPendingDevice) -> Self {
        Self {
            id: value.device_id,
            name: value.name,
            state: SharingState::Pending,
        }
    }
}

impl From<api::FolderConfiguration> for Folder {
    fn from(value: api::FolderConfiguration) -> Self {
        let mut hm = HashMap::new();
        for d in value.devices {
            hm.insert(d.device_id, FolderDeviceSharingDetails::new());
        }
        Self {
            id: value.id,
            label: value.label,
            path: value.path,
            state: SharingState::Configured,
            shared_with: hm,
        }
    }
}

impl From<api::AddedPendingFolder> for Folder {
    fn from(value: api::AddedPendingFolder) -> Self {
        let mut hm = HashMap::new();
        hm.insert(
            value.device_id,
            FolderDeviceSharingDetails::new_pending(Some(value.folder_label)),
        );
        Self {
            id: value.folder_id,
            label: String::new(),
            path: String::new(),
            state: SharingState::Pending,
            shared_with: hm,
        }
    }
}

impl From<Folder> for api::FolderConfiguration {
    fn from(value: Folder) -> Self {
        let mut devices = vec![];
        for (device_id, _info) in value.shared_with {
            devices.push(FolderDevice {
                device_id,
                introduced_by: String::new(),
                encryption_password: String::new(),
            });
        }
        Self {
            id: value.id,
            label: value.label,
            path: value.path,
            devices,
            xattr_filter: XattrFilter::default(),
        }
    }
}
