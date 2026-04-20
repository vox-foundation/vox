use std::marker::PhantomData;

/// An arena-based garbage collector dedicated to a single actor.
/// This prevents global tracing pauses and minimizes K-Complexity for memory bounds.
pub struct ActorHeap {
    /// Total bytes currently allocated in the arena
    allocated_bytes: usize,
    /// Threshold at which a collection should trigger
    next_collection_threshold: usize,
    // Note: A complete implementation integrates a Bumpalo or Semi-Space blocks here.
}

impl ActorHeap {
    /// Creates a new garbage-collected arena targeting an isolated actor.
    pub fn new() -> Self {
        Self {
            allocated_bytes: 0,
            next_collection_threshold: 1024 * 1024, // 1MB threshold initial
        }
    }

    /// Check if the heap has crossed its byte threshold for a required collection.
    pub fn should_collect(&self) -> bool {
        self.allocated_bytes >= self.next_collection_threshold
    }

    /// Perform a localized, non-global sweep on this single actor's heap.
    pub fn collect(&mut self) {
        // Prototype memory reset. Plugs into the actual traversal graph in subsequent waves.
        self.allocated_bytes = 0;
        self.next_collection_threshold = self.next_collection_threshold.saturating_mul(2);
    }

    /// Allocate a value onto the actor's heap, returning an un-sendable pointer.
    pub fn allocate<T>(&mut self, _value: T) -> Gc<T> {
        self.allocated_bytes += std::mem::size_of::<T>();
        Gc {
            _marker: PhantomData,
        }
    }
}

impl Default for ActorHeap {
    fn default() -> Self {
        Self::new()
    }
}

/// A garbage-collected pointer locked to a specific actor's local heap.
///
/// The `!Send` and `!Sync` constraints guarantee that an LLM-generated actor
/// cannot accidentally or maliciously leak this pointer through a mailbox,
/// upholding the shared-nothing capability isolation.
pub struct Gc<T> {
    // using a raw pointer makes the struct !Send and !Sync natively.
    _marker: PhantomData<*mut T>,
}

/// Trait used to bound cross-actor message sends.
/// Because `Gc<T>` cannot be sent across boundaries, structs must implement this
/// to safely copy data into an un-owned buffer before entering the mailbox.
pub trait DeepCloneToOwned {
    type Owned;
    /// Creates a deep, fully owned copy of the underlying data,
    /// completely severing ties to the originating `ActorHeap`.
    fn deep_clone_to_owned(&self) -> Self::Owned;
}

/// Trait to notify unmanaged resources (like GPU tensors) that they have been
/// swept by the ActorHeap garbage collector, so they can manually decrement
/// their external reference counts.
pub trait GcDrop {
    fn gc_drop(&mut self);
}
