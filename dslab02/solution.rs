use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::sync::atomic::AtomicBool;

type Task = Box<dyn FnOnce() + Send>;

struct Worker {
    join_handle: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Task>>>, pool_finished: Arc<AtomicBool>) -> Worker {
        let handle = std::thread::spawn(move || loop {
            let res = receiver.lock().unwrap().recv().unwrap();
            let is_finished = pool_finished.load(std::sync::atomic::Ordering::Relaxed);
            if is_finished {
                println!("worker konczy!");
                break;
            } else {
                res();
            }
            // todo sprawdzić tego unwrapa
        });
        Worker {
            join_handle: Some(handle)
        }
    }

    fn drop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            handle.join().expect("failed to join thread");
        }
    }
}

/// The thread pool.
pub struct Threadpool {
    worker_threads: Vec<Worker>,
    sender: mpsc::Sender<Task>,
    pool_finished: Arc<AtomicBool>,
}

impl Threadpool {
    /// Create new thread pool with `workers_count` workers.
    pub fn new(workers_count: usize) -> Self {
        let (init_sender, r) = mpsc::channel();
        let init_receiver = Arc::new(Mutex::new(r));
        let mut init_workers = Vec::new();
        let init_pool_finished = Arc::new(AtomicBool::new(false));

        for _ in 0..workers_count {
            init_workers.push(Worker::new(Arc::clone(&init_receiver), Arc::clone(&init_pool_finished)));
        }
        
        Threadpool {
            worker_threads: init_workers,
            sender: init_sender,
            pool_finished: init_pool_finished
        }
    }

    /// Submit a new task.
    pub fn submit(&self, task: Task) {
        self.sender.send(Box::new(task)).expect("Thread finished")
    }
}

impl Drop for Threadpool {
    /// Gracefully end the thread pool.
    ///
    /// It waits until all submitted tasks are executed,
    /// and until all threads are joined.
    fn drop(&mut self) {
        for worker in self.worker_threads.iter_mut() {
            worker.drop();
        }
        self.pool_finished.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
