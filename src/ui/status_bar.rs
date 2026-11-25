use crate::error::AppError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct StatusEntry {
    pub severity: Severity,
    pub text: String,
    pub at: chrono::DateTime<chrono::Local>,
}

#[derive(Default)]
pub struct StatusLog {
    entries: Vec<StatusEntry>,
}

impl StatusLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, _severity: Severity, _text: impl Into<String>) {}

    pub fn report(&mut self, _err: &AppError) {}

    pub fn latest(&self) -> Option<&StatusEntry> {
        self.entries.last()
    }

    pub fn entries(&self) -> &[StatusEntry] {
        &self.entries
    }
}
