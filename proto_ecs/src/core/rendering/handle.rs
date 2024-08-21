use std::{cell::RefCell, mem::MaybeUninit};

use scc::Stack;
use num::{Integer, Zero};

/// Handles for resources like buffers and shaders.
/// We use a concrete type to ensure that resource handles are always of the 
/// same type no matter the backend
#[derive(Debug, Clone, Copy)]
pub struct GenericHandle<IndexType, GenType> 
    where  IndexType : Integer,
            GenType : Integer + Zero
{
    pub(super) index : IndexType,
    pub(super) generation : GenType
}

pub type Handle = GenericHandle<u32, u32>;

pub trait IsHandle : Clone + Copy {
    type Index : Integer + Into<usize> + From<usize> + Clone + Copy; 
    type Generation : Integer + Zero + Clone + Copy; 
    fn index(&self) -> Self::Index;
    fn generation(&self) -> Self::Generation;
    fn new(index : Self::Index, generation : Self::Generation) -> Self;
}

impl<IndexType, GenType> IsHandle for GenericHandle<IndexType, GenType> 
    where  IndexType : Integer + Into<usize> + From<usize> + Clone + Copy,
            GenType : Integer + Clone + Copy
{
    type Generation = GenType;
    type Index = IndexType;

    fn new(index : Self::Index, generation : Self::Generation) -> Self {
        GenericHandle{
            index, 
            generation
        }
    }

    fn generation(&self) -> Self::Generation {
        self.generation
    }

    fn index(&self) -> Self::Index {
        self.index
    }
}

pub struct GenerationalIndexAllocator <K : IsHandle, V>
{
    free: Stack<usize>,
    entries: Vec<AllocatorEntry<V, K::Generation>>
}

struct AllocatorEntry<V, G> {
    value : RefCell<MaybeUninit<V>>,
    generation : G
}

impl<K : IsHandle, V> Default for GenerationalIndexAllocator<K,V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K : IsHandle, V> GenerationalIndexAllocator<K, V> {
    const INITIAL_SIZE : usize = 1_000;

    fn new() -> Self {
        GenerationalIndexAllocator {
            free: Stack::default(),
            entries: Vec::with_capacity(Self::INITIAL_SIZE)
        }
    }

    fn allocate(&mut self, value : V) -> K {
        if self.free.is_empty() {
            let next_index = self.entries.len();
            // Allocate a new entry
            self.entries.push(AllocatorEntry{value: RefCell::new(MaybeUninit::new(value)), generation : <K as IsHandle>::Generation::zero()});

            // Create handle:
            return  K::new(K::Index::from(next_index), K::Generation::zero());
        }

        let next_index = **self.free.pop().unwrap();
        let next_generation = self.entries[next_index].generation;
        self.entries[next_index].value.borrow_mut().write(value);

        K::new(next_index.into(), next_generation)
    }

    #[inline(always)]
    fn is_live(&self, key : K) -> bool {
        let index : usize = key.index().into();
        self.entries[index].generation == key.generation()
    }

    fn free(&mut self, key : K) {
        debug_assert!(self.is_live(key), "Trying to access dead handle");

        // Reset the entry
        let index : usize = key.index().into();
        unsafe {
            self.entries[index].value.borrow_mut().assume_init_drop();
        }
        self.entries[index].generation.inc();

        // Add to the free stack again
        self.free.push(index);
    }

    fn get(&self, key : K) -> &mut V {
        debug_assert!(self.is_live(key), "Trying to access dead handle");
        let index : usize = key.index().into();
        let entry = self.entries[index];
        return unsafe {
            entry.value.borrow_mut().as_mut_ptr().as_mut().unwrap()
        }
    }
}