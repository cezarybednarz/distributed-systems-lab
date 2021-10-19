use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::sync::atomic::AtomicBool;

type Task = Box<dyn FnOnce() + Send>;

struct Worker {
    join_handle: JoinHandle<()>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Task>>>) -> Worker {
        let handle = std::thread::spawn(move || loop {
            let res = receiver.lock().unwrap().recv().unwrap();
            println!("dostałem joba!!!");
            // todo sprawdzić tego unwrapa
            res();
        });
        Worker {
            join_handle: handle
        }
    }
}

/// The thread pool.
pub struct Threadpool {
    worker_threads: Vec<Worker>,
    sender: mpsc::Sender<Task>,
    finished: AtomicBool,
}

impl Threadpool {
    /// Create new thread pool with `workers_count` workers.
    pub fn new(workers_count: usize) -> Self {
        let (init_sender, r) = mpsc::channel();
        let init_receiver = Arc::new(Mutex::new(r));
        let mut init_workers = Vec::new();

        for _ in 0..workers_count {
            init_workers.push(Worker::new(Arc::clone(&init_receiver)));
        }
        
        Threadpool {
            worker_threads: init_workers,
            sender: init_sender,
            finished: AtomicBool::new(false)
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
        // todo joiny
        self.finished.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
