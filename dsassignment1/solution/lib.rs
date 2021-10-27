use std::time::Duration;

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
pub struct System {}

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
        unimplemented!()
    }

    /// Creates and starts a new instance of the system.
    pub async fn new() -> Self {
        unimplemented!()
    }

    /// Gracefully shuts the system down.
    pub async fn shutdown(&mut self) {
        unimplemented!()
    }
}

/// A reference to a module used for sending messages.
// You can add fields to this struct.
pub struct ModuleRef<T: Send + 'static> {
    // A dummy marker to satisfy the compiler. It can be removed if type T is
    // used in some other field.
    pub(crate) mod_internal: std::marker::PhantomData<T>,
}

impl<T: Send> ModuleRef<T> {
    /// Sends the message to the module.
    pub async fn send<M: Message>(&self, msg: M)
    where
        T: Handler<M>,
    {
        unimplemented!()
    }
}

impl<T: Send> Clone for ModuleRef<T> {
    /// Creates a new reference to the same module.
    fn clone(&self) -> Self {
        unimplemented!()
    }
}
