//! In-memory [`MessageBus`] for local A2A delivery.

mod message_bus;
#[cfg(test)]
mod tests;

pub use message_bus::MessageBus;
