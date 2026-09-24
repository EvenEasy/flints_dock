use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", content = "reason", rename_all = "snake_case")]
pub enum ScanStatus {
    Complete,
    Partial(String),
    Unsupported(String),
    Failed(String),
    Skipped(String),
}

impl ScanStatus {
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

#[derive(Debug, Serialize)]
pub struct ScanCollection<T> {
    pub status: ScanStatus,
    pub items: Vec<T>,
    pub unknown: Vec<super::UnknownAsset>,
}

impl<T> ScanCollection<T> {
    pub fn complete(items: Vec<T>) -> Self {
        Self {
            status: ScanStatus::Complete,
            items,
            unknown: Vec::new(),
        }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            status: ScanStatus::Failed(reason.into()),
            items: Vec::new(),
            unknown: Vec::new(),
        }
    }

    pub fn issue(&mut self, reason: impl Into<String>) {
        let reason = reason.into();
        match &mut self.status {
            ScanStatus::Partial(message) => {
                message.push_str("; ");
                message.push_str(&reason);
            }
            _ => self.status = ScanStatus::Partial(reason),
        }
    }
}
