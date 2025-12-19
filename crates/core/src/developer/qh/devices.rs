use serde::Deserialize;
use plist::{Dictionary, Date, Value};

use crate::Error;

use crate::developer_endpoint;
use super::{DeveloperSession, ResponseMeta};

impl DeveloperSession {
    pub async fn qh_list_devices(&self, team_id: &String) -> Result<DevicesResponse, Error> {
        let endpoint = developer_endpoint!("/QH65B2/ios/listDevices.action");
        
        let mut body = Dictionary::new();
        body.insert("teamId".into(), Value::String(team_id.clone()));

        let response = self.qh_send_request(&endpoint, Some(body)).await?;
        let response_data: DevicesResponse = plist::from_value(&Value::Dictionary(response))?;

        Ok(response_data)
    }

    pub async fn qh_add_device(&self, team_id: &String, device_name: &String, device_udid: &String) -> Result<DeviceResponse, Error> {
        let endpoint = developer_endpoint!("/QH65B2/ios/addDevice.action");
        
        let mut body = Dictionary::new();
        body.insert("teamId".into(), Value::String(team_id.clone()));
        body.insert("name".into(), Value::String(device_name.clone()));
        body.insert("deviceNumber".into(), Value::String(device_udid.clone()));

        let response = self.qh_send_request(&endpoint, Some(body)).await?;
        let response_data: DeviceResponse = plist::from_value(&Value::Dictionary(response))?;
        
        Ok(response_data)
    }

    pub async fn qh_get_device(&self, team_id: &String, device_udid: &String) -> Result<Option<Device>, Error> {
        let response_data = self.qh_list_devices(team_id).await?;
        
        let device = response_data.devices.into_iter()
            .find(|dev| dev.device_number == *device_udid);

        Ok(device)
    }

    pub async fn qh_ensure_device(&self, team_id: &String, device_name: &String, device_udid: &String) -> Result<Device, Error> {
        if let Some(device) = self.qh_get_device(team_id, device_udid).await? {
            Ok(device)
        } else {
            let response = self.qh_add_device(team_id, device_name, device_udid).await?;
            Ok(response.device)
        }
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DevicesResponse {
    pub devices: Vec<Device>,
    #[serde(flatten)]
    pub meta: ResponseMeta,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceResponse {
    pub device: Device,
    #[serde(flatten)]
    pub meta: ResponseMeta,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    device_id: String,
    name: String,
    device_number: String,
    device_platform: String,
    status: String,
    device_class: String,
    expiration_date: Option<Date>,
}
