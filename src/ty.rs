use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    folders: Vec<Folder>,
    devices: Vec<Device>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Folder {
    id: String,
    label: String,
    path: String,
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
