use crate::ocean_index::error::RuntimeError;
use crate::ocean_index::job_queue::{FileJob, JobPriority, JobQueue};

#[test]
fn job_queue_empty_on_creation() {
    let q = JobQueue::new(100);
    assert!(q.is_empty());
    assert_eq!(q.len(), 0);
    assert!(!q.has_backlog());
}

#[test]
fn job_queue_enqueue_dequeue_high_priority() {
    let mut q = JobQueue::new(100);
    let job = FileJob {
        file_id: "id1".into(),
        path: "/a".into(),
        priority: JobPriority::High,
        retry_count: 0,
    };
    q.enqueue(job).unwrap();
    assert_eq!(q.len(), 1);
    let popped = q.dequeue().unwrap();
    assert_eq!(popped.path, "/a");
    assert_eq!(popped.priority, JobPriority::High);
    assert!(q.is_empty());
}

#[test]
fn job_queue_priority_order() {
    let mut q = JobQueue::new(100);
    q.enqueue(FileJob { file_id: "low".into(), path: "/low".into(), priority: JobPriority::Low, retry_count: 0 }).unwrap();
    q.enqueue(FileJob { file_id: "normal".into(), path: "/normal".into(), priority: JobPriority::Normal, retry_count: 0 }).unwrap();
    q.enqueue(FileJob { file_id: "high".into(), path: "/high".into(), priority: JobPriority::High, retry_count: 0 }).unwrap();

    assert_eq!(q.dequeue().unwrap().path, "/high");
    assert_eq!(q.dequeue().unwrap().path, "/normal");
    assert_eq!(q.dequeue().unwrap().path, "/low");
    assert!(q.is_empty());
}

#[test]
fn job_queue_normal_fallback() {
    let mut q = JobQueue::new(100);
    q.enqueue(FileJob { file_id: "n1".into(), path: "/n1".into(), priority: JobPriority::Normal, retry_count: 0 }).unwrap();
    q.enqueue(FileJob { file_id: "n2".into(), path: "/n2".into(), priority: JobPriority::Normal, retry_count: 0 }).unwrap();
    assert_eq!(q.dequeue().unwrap().path, "/n1");
    assert_eq!(q.dequeue().unwrap().path, "/n2");
}

#[test]
fn job_queue_empty_dequeue() {
    let mut q = JobQueue::new(100);
    assert!(q.dequeue().is_none());
}

#[test]
fn job_queue_capacity_limit() {
    let mut q = JobQueue::new(2);
    q.enqueue(FileJob { file_id: "a".into(), path: "/a".into(), priority: JobPriority::Normal, retry_count: 0 }).unwrap();
    q.enqueue(FileJob { file_id: "b".into(), path: "/b".into(), priority: JobPriority::Normal, retry_count: 0 }).unwrap();
    let err = q.enqueue(FileJob { file_id: "c".into(), path: "/c".into(), priority: JobPriority::Normal, retry_count: 0 }).unwrap_err();
    match err {
        RuntimeError::QueueFull(size) => assert_eq!(size, 2),
        _ => panic!("expected QueueFull"),
    }
}

#[test]
fn job_queue_backlog_detection() {
    let mut q = JobQueue::new(5);
    assert!(!q.has_backlog());
    for i in 0..5 {
        q.enqueue(FileJob { file_id: i.to_string(), path: format!("/{}", i), priority: JobPriority::Normal, retry_count: 0 }).unwrap();
    }
    assert!(q.has_backlog());
}

#[test]
fn job_queue_dequeue_batch() {
    let mut q = JobQueue::new(100);
    for i in 0..10 {
        let priority = if i < 3 { JobPriority::High } else if i < 6 { JobPriority::Normal } else { JobPriority::Low };
        q.enqueue(FileJob { file_id: i.to_string(), path: format!("/{}", i), priority, retry_count: 0 }).unwrap();
    }
    let batch = q.dequeue_batch(5);
    assert_eq!(batch.len(), 5);
    // first 3 high, then 2 normal
    assert_eq!(batch[0].path, "/0");
    assert_eq!(batch[1].path, "/1");
    assert_eq!(batch[2].path, "/2");
    assert_eq!(batch[3].path, "/3");
    assert_eq!(batch[4].path, "/4");
}

#[test]
fn job_queue_dequeue_batch_respects_empty() {
    let mut q = JobQueue::new(100);
    let batch = q.dequeue_batch(10);
    assert!(batch.is_empty());
}

#[test]
fn job_queue_clear() {
    let mut q = JobQueue::new(100);
    q.enqueue(FileJob { file_id: "a".into(), path: "/a".into(), priority: JobPriority::High, retry_count: 0 }).unwrap();
    q.enqueue(FileJob { file_id: "b".into(), path: "/b".into(), priority: JobPriority::Normal, retry_count: 0 }).unwrap();
    q.clear();
    assert!(q.is_empty());
}

#[test]
fn job_queue_enqueue_batch() {
    let mut q = JobQueue::new(100);
    let jobs: Vec<FileJob> = (0..5).map(|i| FileJob {
        file_id: i.to_string(),
        path: format!("/{}", i),
        priority: JobPriority::Low,
        retry_count: 0,
    }).collect();
    q.enqueue_batch(jobs).unwrap();
    assert_eq!(q.len(), 5);
}
