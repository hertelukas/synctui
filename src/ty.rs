use std::collections::HashMap;

use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// Represents the Syncthing configuration XML object.
#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub folders: Vec<Folder>,
    pub devices: Vec<Device>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub label: String,
    pub path: String,
    pub devices: Vec<FolderDevice>,
    xattr_filter: XattrFilter,
}

impl Folder {
    pub fn new(id: String, label: String, path: String, devices: Vec<FolderDevice>) -> Self {
        Self {
            id,
            label,
            path,
            devices,
            xattr_filter: XattrFilter::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct XattrFilter {
    entries: Vec<String>,
    max_single_entry_size: u64,
    max_total_size: u64,
}

impl Default for XattrFilter {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            max_single_entry_size: 1024,
            max_total_size: 4096,
        }
    }
}

/// Representing devices with which we share a folder
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderDevice {
    #[serde(rename = "deviceID")]
    pub device_id: String,
    introduced_by: String,
    encryption_password: String,
}

impl FolderDevice {
    pub fn new(id: &str) -> Self {
        Self {
            device_id: id.to_string(),
            introduced_by: String::new(),
            encryption_password: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    #[serde(rename = "deviceID")]
    pub device_id: String,
    pub name: String,
    pub addresses: Vec<String>, // TODO parse as SocketAddr or "dynamic"
    compression: Compression,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
enum Compression {
    #[default]
    Metadata,
    Always,
    Never,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: u64,
    #[serde(rename = "globalID")]
    global_id: u64,
    time: chrono::DateTime<Utc>,
    #[serde(flatten)]
    pub ty: EventType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum EventType {
    ClusterConfigReceived {},
    ConfigSaved {},
    #[serde(rename_all = "camelCase")]
    DeviceConnected {
        addr: String,
        id: String,
        device_name: String,
        client_name: String,
        client_version: String,
        #[serde(rename = "type")]
        ty: ConnectionType,
    },
    DeviceDisconnected {
        error: String,
        id: String,
    },
    DeviceDiscovered {},
    DevicePause {},
    DeviceRejected {}, // Deprecated
    DeviceResumed {},
    DownloadProgress {},
    Failure {},
    FolderCompletion {},
    FolderErrors {},
    FolderPaused {},
    FolderRejected {}, // Deprecated
    FolderResumed {},
    FolderScanProgress {},
    FolderSummary {},
    FolderWatchStateChanged {},
    ItemFinished {},
    ItemStarted {},
    ListenAddressesChanged {},
    LocalChangeDetected {},
    LocalIndexUpdated {},
    LoginAttempt {},
    PendingDevicesChanged {
        added: Option<Vec<AddedPendingDevice>>,
        removed: Option<Vec<RemovedPendingDevice>>,
    },
    PendingFoldersChanged {},
    RemoteChangeDetected {},
    RemoteDownloadProgress {},
    RemoteIndexUpdated {},
    Starting {},
    StartupComplete {},
    StateChanged {},
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ConnectionType {
    #[serde(rename = "tcp-client")]
    TCPClient,
    #[serde(rename = "tcp-server")]
    TCPServer,
    #[serde(rename = "relay-client")]
    RelayClient,
    #[serde(rename = "relay-server")]
    RelayServer,
    #[serde(rename = "quic-server")]
    QuicServer,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct AddedPendingDevice {
    pub address: std::net::SocketAddr,
    #[serde(rename = "deviceID")]
    pub device_id: String,
    pub name: String,
}

impl Into<Device> for AddedPendingDevice {
    fn into(self) -> Device {
        Device {
            device_id: self.device_id,
            name: self.name,
            // TODO
            addresses: vec!["dynamic".to_string()],
            compression: Compression::default(),
        }
    }
}

impl std::fmt::Display for AddedPendingDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "\"{}\" ({} at {})",
            self.name, self.device_id, self.address
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RemovedPendingDevice {
    #[serde(rename = "deviceID")]
    device_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PendingDevices {
    #[serde(flatten)]
    devices: HashMap<String, PendingDevice>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PendingDevice {
    time: chrono::DateTime<Utc>,
    name: String,
    address: std::net::SocketAddr,
}
