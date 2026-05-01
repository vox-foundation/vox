#![cfg(test)]

use crate::mens::hardware::probe::{HardwareProbe, ProbeError};
use crate::mens::hardware::types::HardwareSummary;
use async_trait::async_trait;

pub(crate) struct MockProbe {
    pub name: &'static str,
    pub applicable: bool,
    pub result: Result<Option<HardwareSummary>, ProbeError>,
}

#[async_trait]
impl HardwareProbe for MockProbe {
    fn name(&self) -> &'static str {
        self.name
    }
    fn applicable(&self) -> bool {
        self.applicable
    }
    async fn probe(&self) -> Result<Option<HardwareSummary>, ProbeError> {
        match &self.result {
            Ok(opt) => Ok(opt.clone()),
            Err(e) => Err(match e {
                ProbeError::LibraryUnavailable(s) => ProbeError::LibraryUnavailable(s.clone()),
                ProbeError::PermissionDenied(s) => ProbeError::PermissionDenied(s.clone()),
                ProbeError::DeviceError(s) => ProbeError::DeviceError(s.clone()),
                ProbeError::Other(s) => ProbeError::Other(s.clone()),
            }),
        }
    }
}
