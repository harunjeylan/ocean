use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;

use crate::ocean_index::config::RateLimiterConfig;
use crate::ocean_index::error::RuntimeError;
use crate::ocean_index::worker_pool::CountingSemaphore;

pub struct RateLimiter {
    semaphore: CountingSemaphore,
    rps: Option<u64>,
    window_timestamps: Mutex<VecDeque<Instant>>,
}

impl RateLimiter {
    pub fn new(config: &RateLimiterConfig) -> Self {
        Self {
            semaphore: CountingSemaphore::new(config.max_concurrent),
            rps: config.requests_per_minute,
            window_timestamps: Mutex::new(VecDeque::new()),
        }
    }

    pub fn acquire(&self) -> Result<PermitGuard<'_>, RuntimeError> {
        if let Some(rps) = self.rps {
            self.enforce_rpm(rps)?;
        }
        let permit = self.semaphore.acquire()?;
        Ok(PermitGuard {
            permit: Some(permit),
            window_timestamps: &self.window_timestamps,
            acquired_at: Instant::now(),
        })
    }

    pub fn try_acquire(&self) -> Result<Option<PermitGuard<'_>>, RuntimeError> {
        if let Some(rps) = self.rps {
            let timestamps = self.window_timestamps.lock().unwrap();
            let cutoff = Instant::now() - std::time::Duration::from_secs(60);
            let count = timestamps.iter().filter(|t| **t > cutoff).count();
            if count as u64 >= rps {
                return Ok(None);
            }
        }
        let permit = self.semaphore.try_acquire();
        match permit {
            Some(p) => {
                Ok(Some(PermitGuard {
                    permit: Some(p),
                    window_timestamps: &self.window_timestamps,
                    acquired_at: Instant::now(),
                }))
            }
            None => Ok(None),
        }
    }

    pub fn available_permits(&self) -> usize {
        let max = self.semaphore.available_permits();
        if let Some(rps) = self.rps {
            let timestamps = self.window_timestamps.lock().unwrap();
            let cutoff = Instant::now() - std::time::Duration::from_secs(60);
            let recent = timestamps.iter().filter(|t| **t > cutoff).count() as u64;
            if recent >= rps {
                return 0;
            }
        }
        max
    }

    fn enforce_rpm(&self, rps: u64) -> Result<(), RuntimeError> {
        loop {
            let mut timestamps = self.window_timestamps.lock().unwrap();
            let cutoff = Instant::now() - std::time::Duration::from_secs(60);
            while let Some(&t) = timestamps.front() {
                if t <= cutoff {
                    timestamps.pop_front();
                } else {
                    break;
                }
            }
            if (timestamps.len() as u64) < rps {
                return Ok(());
            }
            drop(timestamps);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

pub struct PermitGuard<'a> {
    permit: Option<super::worker_pool::SemaphorePermit>,
    window_timestamps: &'a Mutex<VecDeque<Instant>>,
    acquired_at: Instant,
}

impl<'a> Drop for PermitGuard<'a> {
    fn drop(&mut self) {
        if self.permit.is_some() {
            if let Ok(mut ts) = self.window_timestamps.lock() {
                ts.push_back(self.acquired_at);
            }
        }
    }
}
