use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub folders: Vec<Folder>,
    pub devices: Vec<Device>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Folder {
    pub id: String,
    pub label: String,
    pub path: String,
    devices: Vec<FolderDevice>,
}

/// Representing devices with which we share a folder
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FolderDevice {
    #[serde(rename = "deviceID")]
    device_id: String,
    introduced_by: String,
    encryption_password: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    #[serde(rename = "deviceID")]
    device_id: String,
    name: String,
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
}
