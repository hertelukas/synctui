///! Types reflecting the [official types](https://github.com/syncthing/syncthing/blob/main/lib/config/config.go)
use std::collections::HashMap;

use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// Represents the Syncthing configuration XML object.
#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    version: u64,
    pub folders: Vec<FolderConfiguration>,
    pub devices: Vec<DeviceConfiguration>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderConfiguration {
    pub id: String,
    pub label: String,
    pub path: String,
    pub devices: Vec<FolderDevice>,
    xattr_filter: XattrFilter,
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceConfiguration {
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
    PendingFoldersChanged {
        added: Option<Vec<AddedPendingFolder>>,
        removed: Option<Vec<RemovedPendingFolder>>,
    },
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RemovedPendingDevice {
    #[serde(rename = "deviceID")]
    device_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PendingDevices {
    #[serde(flatten)]
    pub devices: HashMap<String, PendingDevice>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PendingDevice {
    time: chrono::DateTime<Utc>,
    pub name: String,
    address: std::net::SocketAddr,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddedPendingFolder {
    #[serde(rename = "deviceID")]
    pub device_id: String,
    #[serde(rename = "folderID")]
    pub folder_id: String,
    pub folder_label: String,
    receive_encrypted: bool,
    remote_encrypted: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RemovedPendingFolder {
    /// A removed entry without `device_id`, means that the folder is
    /// no longer pending on any device.
    #[serde(rename = "deviceID")]
    device_id: Option<String>,
    #[serde(rename = "folderID")]
    folder_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PendingFolders {
    #[serde(flatten)]
    pub folders: HashMap<String, PendingFolder>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PendingFolder {
    /// Maps deviceID to the information about that folder on that device
    pub offered_by: HashMap<String, PendingFolderOfferer>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PendingFolderOfferer {
    time: chrono::DateTime<Utc>,
    pub label: String,
    receive_encrypted: bool,
    remote_encrypted: bool,
}
