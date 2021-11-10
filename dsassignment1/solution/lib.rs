use std::{rc::Rc, time::Duration};
use tokio::task::JoinHandle;
use async_channel::{Receiver, Sender, unbounded};

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

/// The message sent as a result of calling `System::request_tick()`.
#[derive(Debug, Clone)]
pub struct Tick {}

// You can add fields to this struct.
pub struct System {
    // handle of the tokio task and sender for shutdown messages
    module_refs: Vec<(JoinHandle<()>, Sender<()>)>,
}

impl System {
    /// Schedules a `Tick` message to be sent to the given module periodically
    /// with the given interval. The first tick is sent immediately.
    pub async fn request_tick<T: Handler<Tick> + Send>(
        &mut self,
        requester: &ModuleRef<T>,
        delay: Duration,
    ) {
        unimplemented!()
    }

    /// Registers the module in the system.
    /// Returns a `ModuleRef`, which can be used then to send messages to the module.
    pub async fn register_module<T: Send + 'static>(&mut self, module: T) -> ModuleRef<T> {
        let (shutdown_tx, shutdown_rx) = unbounded();
        let (message_tx, message_rx) = unbounded();
        let module_ref = ModuleRef {
            message_tx,
            shutdown_rx: shutdown_rx.clone(),
        };

        let module_handle = tokio::spawn(async move {
            let mut mut_mod = module;
            loop {
                match shutdown_rx.try_recv() {
                    Err(_) => {
                        message_rx.recv().await.unwrap().get_handled(&mut mut_mod).await;
                    }
                    Ok(_) => break,
                }
            }
        });

        self.module_refs.push((module_handle, shutdown_tx));
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
        for (module_handle, shutdown_tx) in self.module_refs.iter_mut() {
            module_handle.await.unwrap();
            shutdown_tx.send(()).await.unwrap();
        }
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
// You can add fields to this struct.
pub struct ModuleRef<T: Send + 'static> {
    message_tx: Sender<Box<dyn Handlee<T>>>,
    shutdown_rx: Receiver<()>,
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
        ModuleRef { message_tx: self.message_tx.clone(), shutdown_rx: self.shutdown_rx.clone() }
    }
}
