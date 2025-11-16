use std::fmt;

use idevice::usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdDevice};
use idevice::lockdown::LockdownClient;
use idevice::IdeviceService;

use crate::Error;

pub const CONNECTION_LABEL: &str = "plume_info";

macro_rules! get_dict_string {
    ($dict:expr, $key:expr) => {
        $dict
            .as_dictionary()
            .and_then(|dict| dict.get($key))
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "".to_string())
    };
}

#[derive(Debug, Clone)]
pub struct Device {
    pub name: String,
    pub uuid: String,
    pub usbmuxd_device: UsbmuxdDevice,
}

impl Device {
    pub async fn new(usbmuxd_device: UsbmuxdDevice) -> Self {
        let name = Self::get_name_from_usbmuxd_device(&usbmuxd_device)
            .await
            .unwrap_or_default();
        
        Device {
            name,
            uuid: usbmuxd_device.udid.clone(),
            usbmuxd_device 
        }
    }
    
    async fn get_name_from_usbmuxd_device(
        device: &UsbmuxdDevice,
    ) -> Result<String, Error> {
        let mut lockdown = LockdownClient::connect(&device.to_provider(UsbmuxdAddr::default(), CONNECTION_LABEL)).await?;
        let values = lockdown.get_value(None, None).await?;
        Ok(get_dict_string!(values, "DeviceName"))
    }
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}",
            match &self.usbmuxd_device.connection_type {
                Connection::Usb => "USB",
                Connection::Network(_) => "WiFi",
                Connection::Unknown(_) => "Unknown",
            },
            self.name
        )
    }
}
