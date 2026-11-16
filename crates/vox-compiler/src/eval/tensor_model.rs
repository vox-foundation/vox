//! Worker-resident `torch.export` (`.pt2`) model handle.
//!
//! Only safe `.pt2` exported programs are permitted; `torch.load` and pickle archives are denied.

/// Opaque exported PyTorch model handle executed in the Python worker on CPU (default).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedModel {
    pub path: String,
    pub device: String,
}

impl ExportedModel {
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            device: "cpu".to_string(),
        }
    }

    #[must_use]
    pub fn to_device(&self, device: impl Into<String>) -> Self {
        Self {
            path: self.path.clone(),
            device: device.into(),
        }
    }
}
