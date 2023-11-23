use bitvec::store::BitStore;
use lazy_static::lazy_static;
use crate::entities::entity::Entity;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::core::locking::RwLock;

use super::entity::EntityID;
use super::entity_spawn_desc::EntitySpawnDescription;

/// Manage entity allocation and storage. 
/// There should be just one global instance of this struct,
/// accessible with `EntityAllocator::get_global()`
#[derive(Debug, Default)]
pub struct EntityAllocator
{
    entries : Vec<Box<EntityEntry>>,
    free : Vec<usize> // TODO: Use some parallel vector instead
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
    generation : AtomicGeneration 
}

type AtomicGeneration = AtomicU32;

#[derive(Debug, Clone, Copy)]
pub struct EntityPtr
{
    index : usize,
    generation: Generation
}

type Generation = u32;

// -- < Implementations > --------------------------------

lazy_static!{
    static ref GLOBAL_ALLOCATOR : RwLock<EntityAllocator> = RwLock::from(EntityAllocator::new());
}

// TODO: We have to find a better way to do this

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
    /// calling: `ptr.init(id, spawn_desc)`` with the result from this function
    pub fn allocate(&mut self) -> EntityPtr
    {
        if self.free.is_empty()
        {
            // Allocate a new entry
            let new_entry = EntityEntry { 
                    header: EntryHeader { generation: AtomicGeneration::ZERO }, 
                    mem: MaybeUninit::<Entity>::uninit()
                };

            // Pointer to return
            let index = self.entries.len();
            self.entries.push(Box::new(new_entry));

            // Create pointer:
            return EntityPtr{index, generation: 0};
        }

        let index = self.free.pop().unwrap();
        let entry = &self.entries[index];
        
        return EntityPtr{
                    index, 
                    generation: entry.header.generation.load(Ordering::Acquire)
                };
    }

    pub fn free(&mut self, entity_ptr: &EntityPtr)
    {
        if !entity_ptr.is_live()
        {
            panic!("Trying to free already unused index");
        }

        let entry  = &mut self.entries[entity_ptr.index];
        entry.header.generation.fetch_add(1, Ordering::Release);
        unsafe { entry.mem.assume_init_drop() };

        self.free.push(entity_ptr.index);
    }

    #[inline(always)]
    pub fn get(&self, entity_ptr: &EntityPtr) -> &Entity
    {
        let entry = &self.entries[entity_ptr.index];
        unsafe { entry.mem.as_ptr().as_ref().unwrap_unchecked() }
    }

    #[inline(always)]
    pub fn get_mut(&mut self, entity_ptr: &EntityPtr) -> &mut Entity
    {
        let entry = &mut self.entries[entity_ptr.index];
        unsafe { entry.mem.as_mut_ptr().as_mut().unwrap_unchecked() }
    }

    #[inline(always)]
    pub fn is_live(&self, entity_ptr: &EntityPtr) -> bool 
    {
        let entry = &self.entries[entity_ptr.index];
        entry.header.generation.load(Ordering::Relaxed) == entity_ptr.generation
    }

    pub fn get_global() -> &'static RwLock<Self>
    {
        &GLOBAL_ALLOCATOR
    }

}

#[macro_export]
macro_rules! get_entity {
    ($id:ident) => {
            proto_ecs::entities::entity_allocator::EntityAllocator::get_global().read().get($id)
    };
}

#[macro_export]
// I don't like the .write() here, is necessary because getting a mutable 
// reference is a mutable operation. Maybe we can workaround this with some 
// interior mutability trick ?
macro_rules! get_entity_mut {
    ($id:ident) => {
            proto_ecs::entities::entity_allocator::EntityAllocator::get_global().write().get_mut($id)
    };
}

impl EntityPtr
{
    /// If the entity pointed to by this pointer is still valid and live
    #[inline(always)]
    pub fn is_live(&self) -> bool
    {
        let alloc = EntityAllocator::get_global().read();
        return alloc.is_live(self);
    }

    /// Initializes this entity using the same init function 
    /// as the entity struct
    pub fn init(&mut self, id: EntityID, spawn_desc : EntitySpawnDescription) 
    {
        let mut alloc = EntityAllocator::get_global().write();
        alloc.entries[self.index].mem.write(Entity::init(id, spawn_desc));
    }
}