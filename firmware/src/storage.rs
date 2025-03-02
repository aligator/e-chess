use anyhow::Result;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsPartitionId};
use esp_idf_sys::EspError;

pub struct Storage<T: NvsPartitionId> {
    nvs: EspNvs<T>,
}

const SETTINGS_NAMESPACE: &str = "e-chess";

impl<T: NvsPartitionId> Storage<T> {
    pub fn new(nvs_partition: EspNvsPartition<T>) -> Result<Self> {
        let nvs =
            EspNvs::new(nvs_partition, SETTINGS_NAMESPACE, true).expect("could not create nvs");
        Ok(Self { nvs })
    }
    pub fn get_str<const N: usize>(&self, key: &str) -> Result<Option<String>> {
        let mut buf = [0u8; N];
        let result = self.nvs.get_str(key, &mut buf)?;

        Ok(result.map(|s| s.to_string()))
    }
    pub fn set_str(&mut self, key: &str, value: &str) -> Result<(), EspError> {
        self.nvs.set_str(key, value)
    }
}
