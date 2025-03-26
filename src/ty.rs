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
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: u64,
    #[serde(rename = "globalID")]
    global_id: u64,
    time: chrono::DateTime<Utc>,
    #[serde(flatten)]
    ty: EventType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
enum EventType {
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
    PendingDevicesChanged {},
    PendingFoldersChanged {},
    RemoteChangeDetected {},
    RemoteDownloadProgress {},
    RemoteIndexUpdated {},
    Starting {},
    StartupComplete {},
    StateChanged {},
}

#[derive(Serialize, Deserialize, Debug)]
enum ConnectionType {
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
