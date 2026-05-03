//! Reactive state-machine instance helpers (Phase G of the Svelte-mineable
//! features plan, ADR-030 spirit).
//!
//! Today's `state_machine` keyword in the compiler emits typed states + events +
//! a pure reducer function stub at
//! [crates/vox-compiler/src/codegen_ts/state_machine_emit.rs][emit]. What was
//! missing was a runtime instance pattern — something an emitted
//! `useFooStateMachine(initial)` hook (in a `component { }`) or a top-level
//! reactive-class instance (in a `.vox.ui` module per ADR-032) could be built on
//! top of without duplicating dispatch / observation logic in every emitted
//! file.
//!
//! This module ships that helper as a generic `ReactiveStateMachine<S, E>`. It
//! holds the current state and a pure reducer; `send(event)` advances the state
//! and returns the new value. The TSX codegen slice that consumes this module
//! lands separately (it requires touching `state_machine_emit.rs` to wire the
//! helper into emitted hooks).
//!
//! [emit]: ../../../crates/vox-compiler/src/codegen_ts/state_machine_emit.rs

use std::sync::{Arc, Mutex};

/// A pure reducer function: `(current_state, event) -> next_state`.
///
/// State machines emitted by the Vox compiler produce reducer functions in this
/// shape. Wrapping them in [`ReactiveStateMachine`] gives the consumer an
/// instance with `state()` and `send()` methods plus interior mutability for
/// dispatch from multiple call sites.
pub type Reducer<S, E> = Arc<dyn Fn(&S, &E) -> S + Send + Sync>;

/// A reactive state-machine instance.
///
/// Holds an `Arc<Mutex<S>>` of the current state and an `Arc<dyn Fn>` reducer.
/// Cheap to clone (both fields are `Arc`), so the same machine can be shared
/// across observers without cloning the state itself.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use vox_runtime::state_machine::ReactiveStateMachine;
///
/// #[derive(Clone, Debug, PartialEq)]
/// enum Light { On, Off }
/// #[derive(Clone, Debug)]
/// enum Toggle { Flip }
///
/// let machine = ReactiveStateMachine::new(
///     Light::Off,
///     Arc::new(|state, _event: &Toggle| match state {
///         Light::On => Light::Off,
///         Light::Off => Light::On,
///     }),
/// );
///
/// assert_eq!(machine.state(), Light::Off);
/// machine.send(&Toggle::Flip);
/// assert_eq!(machine.state(), Light::On);
/// machine.send(&Toggle::Flip);
/// assert_eq!(machine.state(), Light::Off);
/// ```
pub struct ReactiveStateMachine<S, E>
where
    S: Clone + Send + 'static,
    E: Send + 'static,
{
    state: Arc<Mutex<S>>,
    reducer: Reducer<S, E>,
}

impl<S, E> Clone for ReactiveStateMachine<S, E>
where
    S: Clone + Send + 'static,
    E: Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            reducer: Arc::clone(&self.reducer),
        }
    }
}

impl<S, E> ReactiveStateMachine<S, E>
where
    S: Clone + Send + 'static,
    E: Send + 'static,
{
    /// Construct a new instance from an initial state and a reducer function.
    pub fn new(initial: S, reducer: Reducer<S, E>) -> Self {
        Self {
            state: Arc::new(Mutex::new(initial)),
            reducer,
        }
    }

    /// Read a clone of the current state.
    ///
    /// Returns a clone because the lock is held only for the duration of the
    /// read. The trade-off is `S: Clone`; for cheap-to-clone enums (the
    /// expected case for state-machine variants) this is essentially free.
    pub fn state(&self) -> S {
        self.state
            .lock()
            .expect("ReactiveStateMachine state lock poisoned")
            .clone()
    }

    /// Apply `event` through the reducer and replace the current state with the
    /// result. Returns the new state.
    pub fn send(&self, event: &E) -> S {
        let mut guard = self
            .state
            .lock()
            .expect("ReactiveStateMachine state lock poisoned");
        let next = (self.reducer)(&guard, event);
        *guard = next.clone();
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Light {
        On,
        Off,
    }

    #[derive(Clone, Debug)]
    enum Toggle {
        Flip,
    }

    fn light_reducer() -> Reducer<Light, Toggle> {
        Arc::new(|state, _event| match state {
            Light::On => Light::Off,
            Light::Off => Light::On,
        })
    }

    #[test]
    fn new_initializes_state() {
        let m = ReactiveStateMachine::<Light, Toggle>::new(Light::Off, light_reducer());
        assert_eq!(m.state(), Light::Off);
    }

    #[test]
    fn send_applies_reducer_and_updates_state() {
        let m = ReactiveStateMachine::<Light, Toggle>::new(Light::Off, light_reducer());
        let after = m.send(&Toggle::Flip);
        assert_eq!(after, Light::On);
        assert_eq!(m.state(), Light::On);
    }

    #[test]
    fn send_returns_the_new_state_value() {
        let m = ReactiveStateMachine::<Light, Toggle>::new(Light::On, light_reducer());
        assert_eq!(m.send(&Toggle::Flip), Light::Off);
    }

    #[test]
    fn clone_shares_state_via_arc() {
        let m = ReactiveStateMachine::<Light, Toggle>::new(Light::Off, light_reducer());
        let m2 = m.clone();
        m.send(&Toggle::Flip);
        // Both handles see the same updated state because the inner Mutex is
        // Arc'd; this is the property the codegen layer relies on for sharing
        // a machine between a parent and its children.
        assert_eq!(m2.state(), Light::On);
    }

    #[test]
    fn multiple_sends_compose() {
        let m = ReactiveStateMachine::<Light, Toggle>::new(Light::Off, light_reducer());
        m.send(&Toggle::Flip);
        m.send(&Toggle::Flip);
        m.send(&Toggle::Flip);
        // Three flips from Off → On → Off → On.
        assert_eq!(m.state(), Light::On);
    }
}
