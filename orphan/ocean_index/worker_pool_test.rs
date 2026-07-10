use crate::ocean_index::error::RuntimeError;
use crate::ocean_index::worker_pool::WorkerPool;

#[test]
fn worker_pool_creation_with_specific_sizes() {
    let pool = WorkerPool::new(2, 1, 3);
    assert_eq!(pool.io_pool.current_num_threads(), 2);
    assert_eq!(pool.cpu_pool.current_num_threads(), 1);
    assert_eq!(pool.ai_semaphore.available_permits(), 3);
}

#[test]
fn worker_pool_default_uses_available_parallelism() {
    let pool = WorkerPool::default();
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    assert_eq!(pool.io_pool.current_num_threads(), cpus * 2);
    assert_eq!(pool.cpu_pool.current_num_threads(), cpus);
    assert_eq!(pool.ai_semaphore.available_permits(), 2);
}

#[test]
fn worker_pool_run_io() {
    let pool = WorkerPool::new(2, 2, 2);
    let result = pool.run_io(|| 42).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn worker_pool_run_cpu() {
    let pool = WorkerPool::new(2, 2, 2);
    let result = pool.run_cpu(|| "hello").unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn worker_pool_run_ai() {
    let pool = WorkerPool::new(2, 2, 2);
    let result = pool.run_ai(|| 99).unwrap();
    assert_eq!(result, 99);
}

#[test]
fn worker_pool_ai_semaphore_capacity() {
    let pool = WorkerPool::new(2, 2, 1);
    let handle = std::thread::spawn(move || {
        let r1 = pool.run_ai(|| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            1
        });
        // this should still succeed because the first one will release
        let r2 = pool.run_ai(|| 2);
        (r1, r2)
    });
    let (r1, r2) = handle.join().unwrap();
    assert!(r1.is_ok());
    assert!(r2.is_ok());
}

#[test]
fn worker_pool_panic_caught_in_io() {
    let pool = WorkerPool::new(2, 2, 2);
    let result = pool.run_io(|| {
        panic!("io panic!");
    });
    match result {
        Err(RuntimeError::PoolPanic(msg)) => assert!(msg.contains("io panic")),
        _ => panic!("expected PoolPanic error"),
    }
}

#[test]
fn worker_pool_panic_caught_in_cpu() {
    let pool = WorkerPool::new(2, 2, 2);
    let result = pool.run_cpu(|| {
        panic!("cpu panic!");
    });
    match result {
        Err(RuntimeError::PoolPanic(msg)) => assert!(msg.contains("cpu panic")),
        _ => panic!("expected PoolPanic error"),
    }
}

#[test]
fn worker_pool_io_returns_result_on_success() {
    let pool = WorkerPool::new(2, 2, 2);
    let r: Result<i32, RuntimeError> = pool.run_io(|| {
        let x = 10;
        let y = 20;
        x + y
    });
    assert_eq!(r.unwrap(), 30);
}
