use std::sync::Condvar;
use std::sync::Mutex;

use crate::ocean_index::error::RuntimeError;

pub struct CountingSemaphore {
    count: Mutex<usize>,
    condvar: Condvar,
}

impl CountingSemaphore {
    pub fn new(max: usize) -> Self {
        Self {
            count: Mutex::new(max),
            condvar: Condvar::new(),
        }
    }

    pub fn acquire(&self) -> Result<SemaphorePermit, RuntimeError> {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.condvar.wait(count).unwrap();
        }
        *count -= 1;
        Ok(SemaphorePermit { sem: self as *const CountingSemaphore })
    }

    pub fn try_acquire(&self) -> Option<SemaphorePermit> {
        let mut count = self.count.lock().unwrap();
        if *count == 0 {
            return None;
        }
        *count -= 1;
        Some(SemaphorePermit { sem: self as *const CountingSemaphore })
    }

    pub fn available_permits(&self) -> usize {
        *self.count.lock().unwrap()
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.condvar.notify_one();
    }
}

pub struct SemaphorePermit {
    sem: *const CountingSemaphore,
}

impl Drop for SemaphorePermit {
    fn drop(&mut self) {
        if !self.sem.is_null() {
            unsafe { (*self.sem).release(); }
        }
    }
}

unsafe impl Send for SemaphorePermit {}
unsafe impl Sync for SemaphorePermit {}

pub struct WorkerPool {
    pub io_pool: rayon::ThreadPool,
    pub cpu_pool: rayon::ThreadPool,
    pub ai_semaphore: CountingSemaphore,
}

impl WorkerPool {
    pub fn new(io_threads: usize, cpu_threads: usize, max_ai_concurrent: usize) -> Self {
        let io_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(io_threads)
            .panic_handler(|_| {})
            .build()
            .expect("failed to build IO thread pool");

        let cpu_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(cpu_threads)
            .panic_handler(|_| {})
            .build()
            .expect("failed to build CPU thread pool");

        let ai_semaphore = CountingSemaphore::new(max_ai_concurrent);

        Self { io_pool, cpu_pool, ai_semaphore }
    }

    pub fn run_io<T: Send>(&self, f: impl FnOnce() -> T + Send) -> Result<T, RuntimeError> {
        self.io_pool.install(|| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            result.map_err(|e| {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                RuntimeError::PoolPanic(msg)
            })
        })
    }

    pub fn run_cpu<T: Send>(&self, f: impl FnOnce() -> T + Send) -> Result<T, RuntimeError> {
        self.cpu_pool.install(|| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            result.map_err(|e| {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                RuntimeError::PoolPanic(msg)
            })
        })
    }

    pub fn run_ai<T: Send>(&self, f: impl FnOnce() -> T + Send) -> Result<T, RuntimeError> {
        let _permit = self.ai_semaphore.acquire()?;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        drop(_permit);
        result.map_err(|e| {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            RuntimeError::PoolPanic(msg)
        })
    }
}

impl Default for WorkerPool {
    fn default() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(cpus * 2, cpus, 2)
    }
}
