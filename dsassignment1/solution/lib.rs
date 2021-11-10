use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use async_channel::{Receiver, Sender, unbounded};
use tokio::time;
use core::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub trait Message: Send + 'static {}
impl<T: Send + 'static> Message for T {}

/// A trait for modules capable of handling messages of type `M`.
#[async_trait::async_trait]
pub trait Handler<M: Message>
where
    M: Message,
{
    /// Handles the message.
    async fn handle(&mut self, msg: M);
}

trait Closable {
    fn rx_close(&self);
}
impl<T> Closable for Receiver<T> {
    fn rx_close(&self) {
        self.close();
    }
}

/// The message sent as a result of calling `System::request_tick()`.
#[derive(Debug, Clone)]
pub struct Tick {}

pub struct System {
    /// Handle of the tokio task, if module is shut down, receiver of module messages
    module_refs: Vec<(JoinHandle<()>, Arc<AtomicBool>, Box<dyn Closable>)>,
}

impl System {
    /// Schedules a `Tick` message to be sent to the given module periodically
    /// with the given interval. The first tick is sent immediately.
    pub async fn request_tick<T: Handler<Tick> + Send>(
        &mut self,
        requester: &ModuleRef<T>,
        delay: Duration,
    ) { 
        let requester_clone = requester.clone();
        let _task_handle = tokio::spawn( async move {
            let mut interval_delay = time::interval(delay);
            interval_delay.tick().await;
            while !requester_clone.shutdown.load(Ordering::Relaxed) { 
                match requester_clone.message_tx.try_send(Box::new(Tick{})) {
                    Ok(()) => {
                        interval_delay.tick().await;
                    }
                    _ => {
                        break;
                    }
                }
                
            }
        });
    }

    /// Registers the module in the system.
    /// Returns a `ModuleRef`, which can be used then to send messages to the module.
    pub async fn register_module<T: Send + 'static>(&mut self, module: T) -> ModuleRef<T> {
        let (message_tx, message_rx) = unbounded();
        let shutdown = Arc::new(AtomicBool::new(false));
        let module_ref = ModuleRef {
            message_tx,
            shutdown: shutdown.clone(),
        };

        let message_rx_clone = message_rx.clone();
        let shutdown_clone = shutdown.clone();

        let module_handle = tokio::spawn(async move {
            let mut mut_mod = module;
            loop {
                match shutdown.load(Ordering::Relaxed) {
                    false => {
                        message_rx.recv().await.unwrap().get_handled(&mut mut_mod).await;
                    }
                    true => {
                        break;
                    }
                }
            }
        });
        self.module_refs.push((module_handle, shutdown_clone, Box::new(message_rx_clone)));
        module_ref
    }

    /// Creates and starts a new instance of the system.
    pub async fn new() -> Self {
        System {
            module_refs: Vec::new(),
        }
    }

    /// Gracefully shuts the system down.
    pub async fn shutdown(&mut self) {
        for (_, shutdown, _) in self.module_refs.iter_mut() {
            shutdown.store(true, Ordering::Relaxed);
        }
        for (_, _, message_rx) in self.module_refs.iter_mut() {
            message_rx.rx_close();
        }
        for (module_handle, _, _) in self.module_refs.iter_mut() {
            let _ = module_handle.await;
        }
        // after shutdown system is in the beginning state
        self.module_refs = Vec::new();
    }
}

// trait mentioned in task's hint
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

/// A reference to a module used for sending messages.
pub struct ModuleRef<T: Send + 'static> {
    message_tx: Sender<Box<dyn Handlee<T>>>,
    shutdown: Arc<AtomicBool>,
}

impl<T: Send> ModuleRef<T> {
    /// Sends the message to the module.
    pub async fn send<M: Message>(&self, msg: M)
    where
        T: Handler<M>,
    {
        self.message_tx.send(Box::new(msg)).await.unwrap();
    }
}

impl<T: Send> Clone for ModuleRef<T> {
    /// Creates a new reference to the same module.
    fn clone(&self) -> Self {
        ModuleRef { message_tx: self.message_tx.clone(), shutdown: self.shutdown.clone() }
    }
}
