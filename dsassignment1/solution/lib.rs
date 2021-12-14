use async_channel::{unbounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{self, Duration};
use tokio::task::JoinHandle;
use log::error;



pub trait Message: Send + 'static {}

impl<T: Send + 'static> Message for T {}

/// A trait for modules capable of handling messages of type M.
#[async_trait::async_trait]
pub trait Handler<M: Message>
    where
        M: Message,
{
    /// Handles the message.
    async fn handle(&mut self, msg: M);
}

/// The message sent as a result of calling System::request_tick().
#[derive(Debug, Clone)]
pub struct Tick {}

pub struct System {
    shutdown: Arc<AtomicBool>,
    tasks: Vec<(JoinHandle<()>,  Box<dyn Closable>)>,
}

impl System {
    /// Schedules a Tick message to be sent to the given module periodically
    /// with the given interval. The first tick is sent immediately.
    pub async fn request_tick<T: Handler<Tick> + Send>(
        &mut self,
        requester: &ModuleRef<T>,
        delay: Duration,
    ) {
        // after the system is shut down, consecutive function calls panic
        // https://moodle.mimuw.edu.pl/mod/forum/discuss.php?d=6381
        // Wojciech Ciszewski:
        // "[...]. Funkcje po shutdownie mogą panikować lub nic nie robić,
        // ale należy zapewnić, że shutdown nie spowoduje panica w żadnym handlerze."
        if self.shutdown.load(Ordering::Relaxed) {
            panic!();
        }

        // take ownership to be able to move into async block
        let shutdown_clone = self.shutdown.clone(); //
        let req_clone = requester.clone();

        // prepare the interval
        let mut interval = time::interval(delay);
        interval.tick().await; // first interval takes 0s

        let task = tokio::spawn(async move {
            while !shutdown_clone.load(Ordering::Relaxed) {
                let res = req_clone.tx.send(Box::new(Tick {})).await;
                match res {
                    Ok(_) => {}, // Tick successfully sent
                    Err(_) => break // Channel is closed => system shut down => finish task
                }
                interval.tick().await;
            }
        });
        // store handle and channel to join and close respectively during shutdown
        self.tasks.push((task, Box::new(requester.clone().tx)));
    }

    /// Registers the module in the system.
    /// Returns a ModuleRef, which can be used then to send messages to the module.
    pub async fn register_module<T: Send + 'static>(&mut self, module: T) -> ModuleRef<T> {
        if self.shutdown.load(Ordering::Relaxed) {
            panic!();
        }
        let mut mut_mod = module;
        let (tx, rx): (Sender<Box<dyn Handlee<T>>>, Receiver<Box<dyn Handlee<T>>>) = unbounded();

        let rx_clone = rx.clone();
        let shutdown_clone = self.shutdown.clone();

        let task = tokio::spawn(async move {
            while !shutdown_clone.load(Ordering::Relaxed) {
                let res = rx_clone.recv().await;
                match res {
                    Ok(msg) => {
                        // system is shut down so do not proceed with message handling
                        if shutdown_clone.load(Ordering::Relaxed) {
                            break; // some messages could still be in the channel but it's not a problem
                        }
                        msg.get_handled(&mut mut_mod).await;
                    },
                    Err(_) => break,
                }
            }
        });
        self.tasks.push((task, Box::new(rx)));
        ModuleRef { tx }
    }

    /// Creates and starts a new instance of the system.
    pub async fn new() -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let tasks = Vec::new();
        System { shutdown, tasks }
    }

    /// Gracefully shuts the system down.
    pub async fn shutdown(&mut self) {
        if self.shutdown.load(Ordering::Relaxed) {
            panic!();
        }
        self.shutdown.store(true, Ordering::Relaxed);
        for (task, rx) in &mut self.tasks {
            rx.close();
            let res = task.await;
            match res {
                Ok(_) => {}, // task successfully joined
                Err(e) => error!("Some join resulted in an error: {}.", e),
            };
        }
    }
}

/// A reference to a module used for sending messages.
pub struct ModuleRef<T: Send + 'static> {
    tx: Sender<Box<dyn Handlee<T>>>,
}

impl<T: Send> ModuleRef<T> {
    /// Sends the message to the module.
    pub async fn send<M: Message>(&self, msg: M)
        where
            T: Handler<M>,
    {
        let res = self.tx.send(Box::new(msg)).await;
        match res {
            Ok(_) => {}, // message successfully sent
            Err(e) => error!("Sending a message by ModuleRef resulted in an error: {}.", e),
        }
        //self.tx.send(Box::new(msg)).await.unwrap();
    }
}

impl<T: Send> Clone for ModuleRef<T> {
    /// Creates a new reference to the same module.
    fn clone(&self) -> Self {
        ModuleRef {
            tx: self.tx.clone(),
        }
    }
}

#[async_trait::async_trait]
trait Handlee<T>: Send + 'static
    where
        T: Send,
{
    async fn get_handled(self: Box<Self>, module: &mut T);
}

#[async_trait::async_trait]
impl<M, T> Handlee<T> for M
    where
        T: Handler<M> + Send,
        M: Message,
{
    async fn get_handled(self: Box<Self>, module: &mut T) {
        module.handle(*self).await
    }
}

// Helper trait to store channel for messages in a system
#[async_trait::async_trait]
trait Closable
{
    fn close(&self) -> bool;
}
#[async_trait::async_trait]
impl<T> Closable for Receiver<Box<dyn Handlee<T>>>
    where
        T: Send
{
    fn close(&self) -> bool {
        self.close()
    }
}
#[async_trait::async_trait]
impl<T> Closable for Sender<Box<dyn Handlee<T>>>
    where
        T: Send
{
    fn close(&self) -> bool {
        self.close()
    }
}