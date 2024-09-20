use std::{cell::RefCell, fmt::Debug, mem::MaybeUninit};

use num::{Integer, Zero};
use scc::Queue;

/// Handles for resources like buffers and shaders.
/// We use a concrete type to ensure that resource handles are always of the
/// same type no matter the backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericHandle<IndexType, GenType>
where
    IndexType: Integer,
    GenType: Integer + Zero,
{
    pub(super) index: IndexType,
    pub(super) generation: GenType,
}

pub type Handle = GenericHandle<u32, u32>;

pub trait IsHandle: Clone + Copy + Debug {
    type Index: Integer + TryInto<usize> + TryFrom<usize> + Clone + Copy + Debug;
    type Generation: Integer + Zero + Clone + Copy + Debug;
    fn index(&self) -> Self::Index;
    fn generation(&self) -> Self::Generation;
    fn new(index: Self::Index, generation: Self::Generation) -> Self;
    fn array_index(&self) -> usize;
}

impl<IndexType, GenType> IsHandle for GenericHandle<IndexType, GenType>
where
    IndexType: Integer + TryInto<usize> + TryFrom<usize> + Clone + Copy + Debug,
    GenType: Integer + Clone + Copy + Debug,
{
    type Generation = GenType;
    type Index = IndexType;

    fn new(index: Self::Index, generation: Self::Generation) -> Self {
        GenericHandle { index, generation }
    }

    fn generation(&self) -> Self::Generation {
        self.generation
    }

    fn index(&self) -> Self::Index {
        self.index
    }

    fn array_index(&self) -> usize {
        unsafe { self.index().try_into().unwrap_unchecked() }
    }
}

/// Use this allocator in most cases. If you really need
/// one with a specific type of pointer, you can use [GenerationalIndexAllocator] with
/// your specific type of Handle
pub type Allocator<V> = GenerationalIndexAllocator<Handle, V>;

/// Basic allocator type that can work for most cases
pub struct GenerationalIndexAllocator<K: IsHandle, V> {
    free: Queue<usize>,
    entries: Vec<AllocatorEntry<V, K::Generation>>,
}

struct AllocatorEntry<V, G> {
    value: RefCell<MaybeUninit<V>>,
    generation: G,
}

// TODO Actual send + sync implementation
unsafe impl<K: Send + IsHandle, V: Send> Send for GenerationalIndexAllocator<K, V> {}
unsafe impl<K: Sync + IsHandle, V: Sync> Sync for GenerationalIndexAllocator<K, V> {}

impl<K: IsHandle + Debug, V> Default for GenerationalIndexAllocator<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: IsHandle, V> GenerationalIndexAllocator<K, V> {
    const INITIAL_SIZE: usize = 1_000;

    pub fn new() -> Self {
        GenerationalIndexAllocator {
            free: Queue::default(),
            entries: Vec::with_capacity(Self::INITIAL_SIZE),
        }
    }

    pub fn allocate(&mut self, value: V) -> K {
        if self.free.is_empty() {
            let next_index = self.entries.len();
            // Allocate a new entry
            self.entries.push(AllocatorEntry {
                value: RefCell::new(MaybeUninit::new(value)),
                generation: <K as IsHandle>::Generation::zero(),
            });

            // Create handle:
            let index = unsafe {
                // This will only crash in non 32 or 64 bits architectures
                K::Index::try_from(next_index).unwrap_unchecked()
            };
            return K::new(index, K::Generation::zero());
        }

        let next_index = **self.free.pop().unwrap();
        let next_generation = self.entries[next_index].generation;
        self.entries[next_index].value.borrow_mut().write(value);

        let index = unsafe { next_index.try_into().unwrap_unchecked() };
        K::new(index, next_generation)
    }

    #[inline(always)]
    pub fn is_live(&self, key: K) -> bool {
        let index: usize = key.array_index();
        self.entries[index].generation == key.generation()
    }

    pub fn free(&mut self, key: K) {
        debug_assert!(self.is_live(key), "Trying to access dead handle");

        // Reset the entry
        let index: usize = key.array_index();
        unsafe {
            self.entries[index].value.borrow_mut().assume_init_drop();
        }
        self.entries[index].generation.inc();

        // Add to the free stack again
        self.free.push(index);
    }

    pub fn get(&self, key: K) -> &mut V {
        debug_assert!(self.is_live(key), "Trying to access dead handle");
        let index: usize = key.array_index();
        let entry = &self.entries[index];
        return unsafe { entry.value.borrow_mut().as_mut_ptr().as_mut().unwrap() };
    }
}
