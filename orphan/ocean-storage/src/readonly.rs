use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct ReadOnlyGuard {
    enabled: AtomicBool,
}

static GLOBAL_READ_ONLY: ReadOnlyGuard = ReadOnlyGuard {
    enabled: AtomicBool::new(false),
};

impl ReadOnlyGuard {
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
        }
    }

    pub fn global() -> &'static Self {
        &GLOBAL_READ_ONLY
    }

    pub fn check_write_allowed(&self) -> Result<(), &'static str> {
        if self.enabled.load(Ordering::Relaxed) {
            Err("write operation denied: read-only mode is active")
        } else {
            Ok(())
        }
    }

    pub fn check_write_allowed_global() -> Result<(), &'static str> {
        Self::global().check_write_allowed()
    }

    pub fn set_global_enabled(enabled: bool) {
        Self::global().set_enabled(enabled);
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn is_read_only(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

impl Default for ReadOnlyGuard {
    fn default() -> Self {
        Self::new(false)
    }
}
