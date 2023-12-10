use crate::core::locking::RwLock;
use crate::entities::entity::Entity;
use bitvec::store::BitStore;
use lazy_static::lazy_static;
use scc::Queue;
use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};

use super::entity::EntityID;
use super::entity_spawn_desc::EntitySpawnDescription;

/// Manage entity allocation and storage.
/// There should be just one global instance of this struct,
/// accessible with `EntityAllocator::get_global()`
#[derive(Debug, Default)]
pub struct EntityAllocator {
    entries: RwLock<Vec<Box<EntityEntry>>>,
    free: FreeQueue,
}

/// A lock-free queue of pointers to locked entities
type FreeQueue = Queue<*mut EntityLock>;

#[derive(Debug)]
#[repr(C)]
struct EntityEntry {
    header: EntryHeader,
    mem: MaybeUninit<EntityLock>,
}

/// A Locked entity
pub type EntityLock = RwLock<Entity>;

#[derive(Debug)]
struct EntryHeader {
    // It's important that this is atomic since it might be accessed from
    // multiple threads
    generation: AtomicGeneration,
    is_initialized: bool,
}

type AtomicGeneration = AtomicU32;

/// A not owning reference to an [Entity]. Use this to access an entity allocated
/// by the [EntityAllocator]. Note that since this pointer does not own the memory,
/// dereferencing it would cause a segfault if the allocator that returned this
/// pointer is dead.
///
/// You can also segfault if the memory you are trying to access is not yet initialized.
/// This can happen if you have not initialized the pointer after allocating it.
///
/// To check if the pointer is initialized you can use `entity_ptr.is_initialized()`.
/// To initialize the pointer use `entity_ptr.init(id, spawn_desc)`
#[derive(Clone, Copy, PartialEq)]
pub struct EntityPtr {
    ptr: *mut EntityLock,
    generation: Generation,
}

type Generation = u32;

// -- < Implementations > --------------------------------

lazy_static! {
    // I use a RwLock for the global allocator because otherwise we can't get mutable
    // references to it.
    static ref GLOBAL_ALLOCATOR : RwLock<EntityAllocator> = RwLock::new(EntityAllocator::new());
}

unsafe impl Send for EntityAllocator {}
unsafe impl Sync for EntityAllocator {}

impl EntityAllocator {
    /// Initial capacity of the [EntityAllocator]
    const INITIAL_CAPACITY: usize = 10_000;

    /// Create a new empty allocator
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::with_capacity(EntityAllocator::INITIAL_CAPACITY)),
            free: FreeQueue::default(),
        }
    }

    /// Allocate an entity and get a pointer for such entity.
    ///
    /// The entity will be uninitialized, you can initialize it by
    /// calling: `ptr.init(id, spawn_desc)` with the result from this function
    pub fn allocate(&mut self) -> EntityPtr {
        if self.free.is_empty() {
            // Allocate a new entry
            let mut new_entry = Box::new(EntityEntry {
                header: EntryHeader {
                    generation: AtomicGeneration::ZERO,
                    is_initialized: false,
                },
                mem: MaybeUninit::uninit(),
            });

            // Pointer to return
            let mut entries = self.entries.write();
            let ptr = new_entry.mem.as_mut_ptr();
            entries.push(new_entry);

            // Create pointer:
            return EntityPtr { ptr, generation: 0 };
        }

        let ptr = self.free.pop().unwrap().cast();
        let entry = unsafe { EntityEntry::from_ptr(ptr) };

        return EntityPtr {
            ptr,
            generation: entry.header.generation.load(Ordering::Acquire),
        };
    }

    /// Free an entity.
    ///
    /// The Drop function will be called and the memory
    /// will remain uninitialized, and therefore the input
    /// pointer will be invalid.\
    ///
    /// You can check if a pointer is valid using `ptr.is_live()`
    /// And you can check if the entity is initialized using `ptr.is_initialized()`
    pub fn free(&mut self, entity_ptr: &EntityPtr) {
        if !entity_ptr.is_live() {
            panic!("Trying to free already unused index");
        }

        let entry = unsafe { EntityEntry::from_ptr(entity_ptr.ptr) };
        entry.header.generation.fetch_add(1, Ordering::Release);

        if entry.header.is_initialized {
            // Don't drop if not initialized
            entry.header.is_initialized = false;
            unsafe { entry.mem.assume_init_drop() };
        }

        self.free.push(entity_ptr.ptr);
    }

    /// Get a reference to the global allocator
    pub fn get_global() -> &'static RwLock<Self> {
        &GLOBAL_ALLOCATOR
    }
}

impl Deref for EntityPtr {
    type Target = EntityLock;

    fn deref(&self) -> &Self::Target {
        debug_assert!(self.is_live(), "Trying to deref invalid entity ptr");
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl DerefMut for EntityPtr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.is_live(), "Trying to deref invalid entity ptr");
        unsafe { self.ptr.as_mut().unwrap() }
    }
}

impl<'a> EntityEntry {
    /// Create an entity entry from an entity ptr
    ///
    /// # Safety
    /// This function assumes that the `ptr` comes from an [EntityEntry]
    /// allocated by an [EntityAllocator]. Also the allocator will free all allocated
    /// entities after dying, so this pointer can only be safely used when you are sure that
    /// the allocator is still alive.
    unsafe fn from_ptr(ptr: *mut EntityLock) -> &'a mut Self {
        ptr.cast::<u8>()
            .sub(std::mem::size_of::<EntityEntry>() - std::mem::size_of::<EntityLock>())
            .cast::<EntityEntry>()
            .as_mut()
            .unwrap()
    }
}

impl EntityPtr {
    /// If the entity pointed to by this pointer is still valid and live
    #[inline(always)]
    pub fn is_live(&self) -> bool {
        let entry = unsafe { EntityEntry::from_ptr(self.ptr) };
        return entry.header.generation.load(Ordering::Acquire) == self.generation;
    }

    /// Initializes this entity using the same init function
    /// as the entity struct
    pub fn init(&mut self, id: EntityID, spawn_desc: EntitySpawnDescription) {
        let entry = unsafe { EntityEntry::from_ptr(self.ptr) };
        entry.mem.write(RwLock::new(Entity::init(id, self.clone(), spawn_desc)));
        entry.header.is_initialized = true;
    }

    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        let entry = unsafe { EntityEntry::from_ptr(self.ptr) };
        entry.header.is_initialized
    }
}

impl PartialEq<*const EntityLock> for EntityPtr {
    fn eq(&self, other: &*const EntityLock) -> bool {
        std::ptr::eq(self.ptr, *other)
    }
}

impl PartialEq<*mut EntityLock> for EntityPtr {
    fn eq(&self, other: &*mut EntityLock) -> bool {
        std::ptr::eq(self.ptr, *other)
    }
}

impl PartialEq<*const EntityLock> for &EntityPtr {
    fn eq(&self, other: &*const EntityLock) -> bool {
        std::ptr::eq(self.ptr, *other)
    }
}

impl PartialEq<*mut EntityLock> for &EntityPtr {
    fn eq(&self, other: &*mut EntityLock) -> bool {
        std::ptr::eq(self.ptr, *other)
    }
}

impl Debug for EntityPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

unsafe impl Sync for EntityPtr {}
unsafe impl Send for EntityPtr {}
