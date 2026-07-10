use std::collections::VecDeque;

use crate::ocean_index::error::RuntimeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone)]
pub struct FileJob {
    pub file_id: String,
    pub path: String,
    pub priority: JobPriority,
    pub retry_count: u32,
}

pub struct JobQueue {
    high: VecDeque<FileJob>,
    normal: VecDeque<FileJob>,
    low: VecDeque<FileJob>,
    max_size: usize,
}

impl JobQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            high: VecDeque::new(),
            normal: VecDeque::new(),
            low: VecDeque::new(),
            max_size,
        }
    }

    pub fn enqueue(&mut self, job: FileJob) -> Result<(), RuntimeError> {
        if self.len() >= self.max_size {
            return Err(RuntimeError::QueueFull(self.len()));
        }
        match job.priority {
            JobPriority::High => self.high.push_back(job),
            JobPriority::Normal => self.normal.push_back(job),
            JobPriority::Low => self.low.push_back(job),
        }
        Ok(())
    }

    pub fn enqueue_batch(&mut self, jobs: Vec<FileJob>) -> Result<(), RuntimeError> {
        for job in jobs {
            self.enqueue(job)?;
        }
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<FileJob> {
        if let Some(job) = self.high.pop_front() {
            return Some(job);
        }
        if let Some(job) = self.normal.pop_front() {
            return Some(job);
        }
        self.low.pop_front()
    }

    pub fn dequeue_batch(&mut self, max: usize) -> Vec<FileJob> {
        let mut batch = Vec::with_capacity(max);
        while batch.len() < max {
            match self.dequeue() {
                Some(job) => batch.push(job),
                None => break,
            }
        }
        batch
    }

    pub fn len(&self) -> usize {
        self.high.len() + self.normal.len() + self.low.len()
    }

    pub fn is_empty(&self) -> bool {
        self.high.is_empty() && self.normal.is_empty() && self.low.is_empty()
    }

    pub fn has_backlog(&self) -> bool {
        self.len() >= self.max_size
    }

    pub fn clear(&mut self) {
        self.high.clear();
        self.normal.clear();
        self.low.clear();
    }
}
