use std::fmt;

use idevice::usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdDevice};
use idevice::lockdown::LockdownClient;
use idevice::IdeviceService;

use crate::Error;
use idevice::usbmuxd::UsbmuxdConnection;
use idevice::house_arrest::HouseArrestClient;
use idevice::afc::opcode::AfcFopenMode;

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

    pub async fn install_pairing_record(&self, identifier: &String, path: &str) -> Result<(), Error> {
        let mut usbmuxd = UsbmuxdConnection::default().await?;

        let mut pairing_file = usbmuxd.get_pair_record(&self.uuid).await?;
        pairing_file.udid = Some(self.uuid.clone());

        let provider = self.usbmuxd_device.to_provider(UsbmuxdAddr::default(), CONNECTION_LABEL);
        let hc = HouseArrestClient::connect(&provider).await?;
        let mut ac = hc.vend_documents(identifier.clone()).await?;
        let mut f = ac.open(path, AfcFopenMode::Wr).await?;

        f.write(&pairing_file.serialize().unwrap()).await?;

        Ok(())
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
