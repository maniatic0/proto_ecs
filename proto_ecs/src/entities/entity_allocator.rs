use bitvec::store::BitStore;
use lazy_static::lazy_static;
use crate::entities::entity::Entity;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::core::locking::RwLock;
use scc::Queue;
use std::ops::{Deref, DerefMut};

use super::entity::EntityID;
use super::entity_spawn_desc::EntitySpawnDescription;

/// Manage entity allocation and storage. 
/// There should be just one global instance of this struct,
/// accessible with `EntityAllocator::get_global()`
#[derive(Debug, Default)]
pub struct EntityAllocator
{
    entries : RwLock<Vec<Box<EntityEntry>>>,
    free : Queue<*mut Entity>
}

#[derive(Debug)]
#[repr(C)]
struct EntityEntry
{
    header : EntryHeader,
    mem : MaybeUninit<Entity>,
}

#[derive(Debug)]
struct EntryHeader
{
    // It's important that this is atomic since it might be accessed from
    // multiple threads
    generation : AtomicGeneration,
    is_initialized : bool
}

type AtomicGeneration = AtomicU32;

#[derive(Debug, Clone, Copy)]
pub struct EntityPtr
{
    ptr : *mut Entity,
    generation: Generation
}

type Generation = u32;

// -- < Implementations > --------------------------------

lazy_static!{
    static ref GLOBAL_ALLOCATOR : EntityAllocator = EntityAllocator::new();
}

unsafe impl Send for EntityAllocator{}
unsafe impl Sync for EntityAllocator{}

impl EntityAllocator
{
    /// Create a new empty allocator
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Allocate an entity and get a pointer for such entity. 
    /// 
    /// The entity will be uninitialized, you can initialize it by 
    /// calling: `ptr.init(id, spawn_desc)` with the result from this function
    pub fn allocate(&mut self) -> EntityPtr
    {
        if self.free.is_empty()
        {
            // Allocate a new entry
            let mut new_entry = Box::new(EntityEntry { 
                header: EntryHeader { 
                    generation: AtomicGeneration::ZERO, 
                    is_initialized: false 
                }, 
                mem: MaybeUninit::<Entity>::uninit()
            });

            // Pointer to return
            let mut entries = self.entries.write();
            let ptr = new_entry.mem.as_mut_ptr();
            entries.push(new_entry);

            // Create pointer:
            return EntityPtr{ptr, generation: 0};
        }

        let ptr = self.free.pop().unwrap().cast();
        let entry = unsafe { EntityEntry::from_ptr(ptr) };
        
        return EntityPtr{
                    ptr, 
                    generation: entry.header.generation.load(Ordering::Acquire)
                };
    }

    pub fn free(&mut self, entity_ptr: &EntityPtr)
    {
        if !entity_ptr.is_live()
        {
            panic!("Trying to free already unused index");
        }

        let entry  = unsafe {EntityEntry::from_ptr(entity_ptr.ptr)};
        entry.header.generation.fetch_add(1, Ordering::Release);
        
        if entry.header.is_initialized
        {   // Don't drop if not initialized
            entry.header.is_initialized = false;
            unsafe { entry.mem.assume_init_drop() };
        }

        self.free.push(entity_ptr.ptr);
    }

    pub fn get_global() -> &'static Self
    {
        &GLOBAL_ALLOCATOR
    }

}

impl Deref for EntityPtr {

    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        debug_assert!(self.is_live(), "Trying to deref invalid entity ptr");
        unsafe
        {
            self.ptr.as_ref().unwrap()
        }
    }
}

impl DerefMut for EntityPtr
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.is_live(), "Trying to deref invalid entity ptr");
        unsafe
        {
            self.ptr.as_mut().unwrap()
        }
    }
}

impl<'a> EntityEntry
{
    unsafe fn from_ptr(ptr: *mut Entity) -> &'a mut Self
    {
        ptr
            .cast::<u8>()
            .sub(std::mem::size_of::<EntityEntry>() - std::mem::size_of::<Entity>())
            .cast::<EntityEntry>()
            .as_mut()
            .unwrap()
    }
}

impl EntityPtr
{
    /// If the entity pointed to by this pointer is still valid and live
    #[inline(always)]
    pub fn is_live(&self) -> bool
    {
        let entry = unsafe {EntityEntry::from_ptr(self.ptr)};
        return entry.header.generation.load(Ordering::Acquire) == self.generation;
    }

    /// Initializes this entity using the same init function 
    /// as the entity struct
    pub fn init(&mut self, id: EntityID, spawn_desc : EntitySpawnDescription) 
    {
        let entry = unsafe { EntityEntry::from_ptr(self.ptr) };
        entry.mem.write(Entity::init(id, spawn_desc));
        entry.header.is_initialized = true;
    }

    #[inline(always)]
    pub fn is_initialized(&self) -> bool
    {
        let entry = unsafe { EntityEntry::from_ptr(self.ptr) };
        entry.header.is_initialized
    }
}