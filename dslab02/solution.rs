use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

type Task = Box<dyn FnOnce() + Send>;

struct Worker {
    join_handle: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Task>>>) -> Worker {
        let handle = std::thread::spawn(move || loop {
            let res = receiver.lock().unwrap().recv();
            match res {
                Ok(next_receiver) => {
                    next_receiver();
                }
                Err(_) => {
                    break;
                }
            }

        });
        Worker {
            join_handle: Some(handle)
        }
    }

    fn drop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            handle.join().expect("Thread should join");
        }
    }
}

/// The thread pool.
pub struct Threadpool {
    worker_threads: Vec<Worker>,
    sender: Arc<Mutex<mpsc::Sender<Task>>>,
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
            sender: Arc::new(Mutex::new(init_sender)),
        }
    }

    /// Submit a new task.
    pub fn submit(&self, task: Task) {
        self.sender.lock().unwrap().send(Box::new(task)).expect("Thread should finish")
    }
}

impl Drop for Threadpool {
    /// Gracefully end the thread pool.
    ///
    /// It waits until all submitted tasks are executed,
    /// and until all threads are joined.
    fn drop(&mut self) {
        let (s, _) = std::sync::mpsc::channel();
        drop(std::mem::replace(&mut self.sender, Arc::new(Mutex::new(s))));
        for worker in self.worker_threads.iter_mut() {
            worker.drop();
        }
    }
}
