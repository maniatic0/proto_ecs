use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering, AtomicUsize};

use bitvec::store::BitStore;
use lazy_static::lazy_static;

use atomic_float::AtomicF64;
use nohash_hasher::IntMap;

use crate::entities::entity::{EntityID, INVALID_ENTITY_ID};

use crate::core::locking::RwLock;
use crate::systems::common::{StageID, STAGE_COUNT};
use crate::systems::global_systems::{GlobalSystem, GlobalSystemID, GlobalSystemRegistry};

use super::{entity::Entity, entity_spawn_desc::EntitySpawnDescription};

use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};

/// We just go up. If we ever run out of them we can think of blocks of IDs per thread and a better allocation system
static ENTITY_COUNT: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(INVALID_ENTITY_ID + 1);

/// Allocate a new Entity ID
pub fn allocate_entity_id() -> EntityID {
    // Note: if we ever need to do something more complex with IDs we can do it here

    ENTITY_COUNT.fetch_add(1, Ordering::AcqRel)
}

/// Deallocate an Entity ID
pub fn deallocate_entity_id(id: EntityID) {
    assert!(id < ENTITY_COUNT.load(Ordering::Acquire));

    // Note: if we ever need to do something more complex with IDs we can do it here
}

/// Entity Creation Queue type used by worlds
pub type EntityCreationQueue = scc::Queue<RwLock<Option<(EntityID, EntitySpawnDescription)>>>;

/// Entity Deletion Queue type used by worlds
pub type EntityDeletionQueue = scc::Queue<EntityID>;

/// Entity locking mechanism inside World
pub type EntityLock = RwLock<Entity>;

/// Entity storage inside World
pub type EntityStorage = Box<EntityLock>;

/// Entity Map Type that holds all the entities in a World
pub type EntityMap = dashmap::DashMap<EntityID, EntityStorage>;

/// Global System Storage inside world
pub type GlobalSystemStorage = Box<dyn GlobalSystem>;

/// Storage for all global systems currently loaded. If not here, 
/// it means that it's not loaded
pub type GlobalSystemMap = RwLock<IntMap<GlobalSystemID, GlobalSystemStorage>>;

#[derive(PartialEq)]
/// World Reference to an Entity
/// Warning: Only valid while the world is still alive. Never copy or change the ptr manually
pub struct EntityWorldRef {
    pub(super) ptr: NonNull<EntityLock>,
}

impl EntityWorldRef {
    pub(super) fn new(entity_ptr: *mut EntityLock) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(entity_ptr) },
        }
    }

    /// Gets the internal ptr from the entity storage
    pub(super) fn entity_storage_to_entity_lock_ptr(
        entity_box: &mut EntityStorage,
    ) -> *mut EntityLock {
        std::ptr::addr_of_mut!(**entity_box)
    }
}

impl Deref for EntityWorldRef {
    type Target = EntityLock;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl DerefMut for EntityWorldRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl PartialEq<*const EntityLock> for EntityWorldRef {
    fn eq(&self, other: &*const EntityLock) -> bool {
        std::ptr::eq(self.ptr.as_ptr(), *other)
    }
}

impl PartialEq<*mut EntityLock> for EntityWorldRef {
    fn eq(&self, other: &*mut EntityLock) -> bool {
        std::ptr::eq(self.ptr.as_ptr(), *other)
    }
}

impl PartialEq<*const EntityLock> for &EntityWorldRef {
    fn eq(&self, other: &*const EntityLock) -> bool {
        std::ptr::eq(self.ptr.as_ptr(), *other)
    }
}

impl PartialEq<*mut EntityLock> for &EntityWorldRef {
    fn eq(&self, other: &*mut EntityLock) -> bool {
        std::ptr::eq(self.ptr.as_ptr(), *other)
    }
}

impl Debug for EntityWorldRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

unsafe impl Sync for EntityWorldRef {}
unsafe impl Send for EntityWorldRef {}

/// Vector with all the entities inside a world or inside a stage in a world (for faster iteration). Do not use at the same time as the entity map
pub type EntitiesVec = RwLock<Vec<EntityWorldRef>>;

/// Array used to count how many entities are subscribed to some global system to know when we have to unload them
pub type GlobalSystemCount = RwLock<Vec<AtomicUsize>>;

/// World Identifier in the Entity System
pub type WorldID = u16;

#[derive(Debug)]
pub struct World {
    id: WorldID,
    delta_time: DeltaTimeAtomicType,
    fixed_delta_time: DeltaTimeAtomicType,
    delta_time_scaling: DeltaTimeAtomicType,
    entities: EntityMap,
    entities_all: EntitiesVec,
    entities_stages: [EntitiesVec; STAGE_COUNT],
    creation_queue: EntityCreationQueue,
    deletion_queue: EntityDeletionQueue,
    global_systems: GlobalSystemMap,
    global_systems_count: GlobalSystemCount
}

impl World {
    /// Number of chunks to use for stepping a stage
    /// Maybe this should be variable based on load
    const CHUNKS_NUM: usize = 20;

    pub(crate) fn new(id: WorldID) -> Self {
        let gs_count = GlobalSystemRegistry::get_global_registry().read().get_global_system_count();
        let mut gs_count_array = Vec::new();
        for _ in 0..gs_count
        {
            gs_count_array.push(AtomicUsize::ZERO);
        }

        Self {
            id,
            delta_time: Default::default(),
            fixed_delta_time: Default::default(),
            delta_time_scaling: AtomicF64::from(1.0),
            entities: Default::default(),
            entities_all: Default::default(),
            entities_stages: core::array::from_fn(|_| Default::default()),
            creation_queue: Default::default(),
            deletion_queue: Default::default(),
            global_systems: Default::default(),
            global_systems_count: RwLock::new(gs_count_array)
        }
    }

    #[inline(always)]
    pub fn get_id(&self) -> WorldID {
        self.id
    }

    /// Current scaled delta time
    #[inline(always)]
    pub fn get_delta_time(&self) -> DeltaTimeType {
        self.delta_time.load(Ordering::Acquire)
    }

    /// Current scaled fixed delta time
    #[inline(always)]
    pub fn get_fixed_delta_time(&self) -> DeltaTimeType {
        self.fixed_delta_time.load(Ordering::Acquire)
    }

    /// Create a new entity based on its spawn description. Note that the entity will spawn at the end of the current stage
    pub fn create_entity(&self, spawn_desc: EntitySpawnDescription) -> EntityID {
        let new_id = allocate_entity_id();
        self.creation_queue
            .push(RwLock::new(Some((new_id, spawn_desc))));
        new_id
    }

    /// Create a new entity based on its spawn description
    fn create_entity_internal(&self, id: EntityID, spawn_desc: EntitySpawnDescription) {
        let mut entity_box = Box::new(RwLock::new(Entity::init(id, spawn_desc)));
        let entity_lock_ptr: *mut EntityLock =
            EntityWorldRef::entity_storage_to_entity_lock_ptr(&mut entity_box);
        let old = self.entities.insert(id, entity_box);
        assert!(
            old.is_none(),
            "Duplicated Entity ID, old entity {:?}",
            old.unwrap()
        );

        // Insert entity for iteration
        {
            let mut entities_all = self.entities_all.write();
            entities_all.push(EntityWorldRef::new(entity_lock_ptr));
        }

        let entity_ref = unsafe { &*(*entity_lock_ptr).data_ptr() };

        for (stage_id, stage_vec) in self.entities_stages.iter().enumerate() {
            let stage_id = stage_id as StageID;
            if entity_ref.is_stage_enabled(stage_id) {
                stage_vec.write().push(EntityWorldRef::new(entity_lock_ptr));
            }
        }

        // Initialize every global system that is not currently loaded
        for &gs_id in entity_ref.get_global_systems()
        {
            {
                let gs_count = self.global_systems_count.write();
                gs_count[gs_id as usize].fetch_add(1, Ordering::Relaxed);
            }
            if self.global_system_is_loaded(gs_id) {
                continue;
            }

            self.load_global_system(gs_id);
        }
    }

    /// Destroy an entity. Note that the entity will be destroyed at the end of the current stage
    pub fn destroy_entity(&self, id: EntityID) {
        self.deletion_queue.push(id);
    }

    /// Destroy an entity
    pub fn destroy_entity_internal(&self, id: EntityID) {
        let prev = self.entities.remove(&id);
        if prev.is_none() {
            println!("Failed to destroy Entity {id}, maybe it was already deleted (?)");
            return;
        }
        let (_id, mut entity_box) = prev.unwrap();

        let entity_lock_ptr: *mut EntityLock =
            EntityWorldRef::entity_storage_to_entity_lock_ptr(&mut entity_box);
        let entity_ref = unsafe { &*(*entity_lock_ptr).data_ptr() };

        // Destroy entity from iteration lists
        {
            let mut entities_all = self.entities_all.write();
            for (index, vec_ref) in entities_all.iter().enumerate() {
                if vec_ref == entity_lock_ptr {
                    entities_all.swap_remove(index);
                    break;
                }
            }
        }

        // Decrease counters for global systems in this entity
        {
            let gs_counts = self.global_systems_count.write();
            for &gs_id in entity_ref.get_global_systems()
            {
                let result = gs_counts[gs_id as usize].fetch_sub(1, Ordering::Relaxed);

                if result == 1 // this was the last entity requiring this gs
                {
                    self.unload_global_system(gs_id);
                }
            }
        }

        for (stage_id, stage_vec) in self.entities_stages.iter().enumerate() {
            let stage_id = stage_id as StageID;
            if entity_ref.is_stage_enabled(stage_id) {
                let mut stage_vec = stage_vec.write();
                for (index, vec_ref) in stage_vec.iter().enumerate() {
                    if vec_ref == entity_lock_ptr {
                        stage_vec.swap_remove(index);
                        break;
                    }
                }
            }
        }

        deallocate_entity_id(id);
    }

    // Update the delta times in this world
    pub(super) fn update_delta_time_internal(
        &self,
        delta_time: DeltaTimeType,
        fixed_delta_time: DeltaTimeType,
    ) {
        let scale = self.delta_time_scaling.load(Ordering::Acquire);

        self.delta_time.store(delta_time * scale, Ordering::Release);
        self.fixed_delta_time
            .store(fixed_delta_time * scale, Ordering::Release);
    }

    /// Updates the scaling factor used for delta times in this world
    /// It is only applied the next frame
    pub fn update_delta_time_scaling(&self, scaling_factor: DeltaTimeType) {
        self.delta_time_scaling
            .store(scaling_factor, Ordering::Release);
    }

    /// Process all entity commands
    fn process_entity_commands(&self) {
        // Process all deletions
        if !self.deletion_queue.is_empty() {
            let mut work: Vec<EntityID> = Vec::new();
            while !self.deletion_queue.is_empty() {
                let pop = self.deletion_queue.pop().unwrap();
                work.push(**pop);
            }

            work.into_par_iter().for_each(|id| {
                self.destroy_entity_internal(id);
            });
        }

        // Process all creations
        if !self.creation_queue.is_empty() {
            let mut work: Vec<(EntityID, EntitySpawnDescription)> = Vec::new();
            while !self.creation_queue.is_empty() {
                let pop = self.creation_queue.pop().unwrap();
                work.push(pop.write().take().unwrap());
            }

            work.into_par_iter().for_each(|(id, spawn_desc)| {
                self.create_entity_internal(id, spawn_desc);
            });
        }
    }

    /// Process a stage in this world
    fn run_stage(&self, stage_id: StageID) {
        // Process all the entity commands before the stage
        self.process_entity_commands();

        // Run Stage in all entities
        let entities_stage = self.entities_stages[stage_id as usize].read();

        if !entities_stage.is_empty() {
            // Nothing to do, no more commands can be created
            // TODO: Check this for Global Systems. They might need to execute?
            return;
        }

        entities_stage
            .par_chunks(World::CHUNKS_NUM)
            .for_each(|map_refs| {
                for map_ref in map_refs {
                    // Note we don't need to take the lock as we are 100% sure rayon is executing disjoint tasks.
                    let entity = unsafe { &mut *map_ref.data_ptr() };

                    // Check if stage is enabled
                    if entity.is_stage_enabled(stage_id) {
                        entity.run_stage(self, stage_id);
                    }
                }
            });

        // Process all the entity commands created in the stage
        self.process_entity_commands();
    }


    fn global_system_is_loaded(&self, global_system_id : GlobalSystemID) -> bool
    {
        self.global_systems.read().contains_key(&global_system_id)
    }

    /// Creates and initializes a new global system
    fn load_global_system(&self, global_system_id : GlobalSystemID)
    {
        debug_assert!(
            !self.global_systems.read().contains_key(&global_system_id), 
            "Global system was already loaded"
        );

        let gs_registry = GlobalSystemRegistry
                    ::get_global_registry()
                    .read();

        let gs = gs_registry.create_by_id(global_system_id);
        self.global_systems.write().insert(global_system_id, gs);
    }

    fn unload_global_system(&self, global_system_id : GlobalSystemID)
    {
        debug_assert!(
            self.global_systems.read().contains_key(&global_system_id), 
            "Global system was already unloaded"
        );

        self.global_systems.write().remove(&global_system_id);
    }

    /// Merge target world into this world
    fn merge_world(&mut self, mut _target: Self) {
        todo!("Implement world merge!")
    }
}

/// Entity System map type of Worlds
pub type WorldMap = dashmap::DashMap<WorldID, World>;

/// Entity System queue type for destroy world commands
pub type WorldDestroyQueue = scc::Queue<WorldID>;

/// Entity System queue type for merge world commands
pub type WorldMergeQueue = scc::Queue<(WorldID, WorldID)>;

/// Entity System atomic type used for deltas
pub type DeltaTimeAtomicType = AtomicF64;

/// Entity System type used for deltas
pub type DeltaTimeType = f64;

#[derive(Debug)]
pub struct EntitySystem {
    pool: ThreadPool,
    delta_time: DeltaTimeAtomicType,
    fixed_delta_time: DeltaTimeAtomicType,
    requested_reset: AtomicBool,
    worlds: WorldMap,
    world_id_counter: AtomicU16,
    destroy_world_queue: WorldDestroyQueue,
    merge_worlds_queue: WorldMergeQueue,
}

impl EntitySystem {
    /// Get the entity system
    pub fn get() -> &'static Self {
        &ENTITY_SYSTEM
    }

    /// Current unscaled delta time
    #[inline(always)]
    pub fn get_delta_time(&self) -> DeltaTimeType {
        self.delta_time.load(Ordering::Acquire)
    }

    /// Current unscaled fixed delta time
    #[inline(always)]
    pub fn get_fixed_delta_time(&self) -> DeltaTimeType {
        self.fixed_delta_time.load(Ordering::Acquire)
    }

    /// Create a new world
    fn create_world_internal(&self, new_id: WorldID) {
        let old = self.worlds.insert(new_id, World::new(new_id));
        assert!(old.is_none(), "World ID collision! Old : {:?}", old);
    }

    /// Create a new world and return its world ID
    pub fn create_world(&self) -> WorldID {
        let new_id = self.world_id_counter.fetch_add(1, Ordering::AcqRel) as WorldID;

        self.create_world_internal(new_id);

        new_id
    }

    /// Destroy a world
    fn destroy_world_internal(&self, id: WorldID) {
        if self.worlds.remove(&id).is_none() {
            println!("Failed to destroy World {id}, maybe it was already destroyed(?)");
        }
    }

    /// Destroy a world and all of its content
    pub fn destroy_world(&self, id: WorldID) {
        self.destroy_world_queue.push(id);
    }

    /// Merge `source` world into the `target` world. This destroys the `source` world
    fn merge_worlds_internal(&self, source: WorldID, target: WorldID) {
        let target_world = self.worlds.get_mut(&target);
        if target_world.is_none() {
            println!(
                "Failed to merge World {source} into World {target} due to missing target world!"
            );
            return;
        }
        let mut target_world = target_world.unwrap();

        let source_world = self.worlds.remove(&source);
        if source_world.is_none() {
            println!(
                "Failed to merge World {source} into World {target} due to missing source world!"
            );
            return;
        }
        let source_world = source_world.unwrap().1;

        target_world.merge_world(source_world);
    }

    /// Merge `source` world into the `target` world. This destroys the `source` world
    pub fn merge_worlds(&self, source: WorldID, target: WorldID) {
        self.merge_worlds_queue.push((source, target));
    }

    /// Process destroy and merge world commands
    fn process_world_command_queues(&self) {
        // Reset if requested
        if self.requested_reset.load(Ordering::Acquire) {
            self.reset_internal();
        }

        // First process all destroy commands
        while !self.destroy_world_queue.is_empty() {
            let world_id = **self.destroy_world_queue.pop().unwrap();
            self.destroy_world_internal(world_id);
        }

        // Second process all the merge commands
        while !self.merge_worlds_queue.is_empty() {
            let (source, target) = **self.merge_worlds_queue.pop().unwrap();
            self.merge_worlds_internal(source, target);
        }
    }

    /// Process a stage for the entity system and all the worlds
    fn process_stage(&self, stage_id: StageID) {
        // Process all commands created before the stage
        self.process_world_command_queues();

        // Process worlds in parallel
        self.pool.install(|| {
            self.worlds.par_iter().for_each(|world| {
                world.run_stage(stage_id);
            });
        });

        // Process all commands created in the stage
        self.process_world_command_queues();
    }

    /// Step the entity system
    pub fn step(&self, new_delta_time: DeltaTimeType, fixed_delta_time: DeltaTimeType) {
        // Set the current unscaled delta time
        self.delta_time.store(new_delta_time, Ordering::Release);
        self.fixed_delta_time
            .store(fixed_delta_time, Ordering::Release);

        // Update delta times in parallel
        self.pool.install(|| {
            self.worlds.par_iter().for_each(|world| {
                world
                    .update_delta_time_internal(self.get_delta_time(), self.get_fixed_delta_time());
            });
        });

        // Go through all the stages
        for stage_id in 0..STAGE_COUNT {
            self.process_stage(stage_id as StageID);
        }
    }

    /// Create a new entity in World `world_id` based on its spawn description. Note that the entity will spawn at the end of the current stage. If the world cannot be found, it returns the spawn desc as an err
    pub fn create_entity(
        &self,
        world_id: WorldID,
        spawn_desc: EntitySpawnDescription,
    ) -> Result<EntityID, EntitySpawnDescription> {
        match self.worlds.get(&world_id) {
            Some(entry) => Ok(entry.create_entity(spawn_desc)),
            None => {
                println!("Failed to create entity due to: Couldn't find World {world_id}!");
                Err(spawn_desc)
            }
        }
    }

    /// Destroy an entity in World `world_id`, if the world and the entity exist. Return true if the world could be found (not that the entity might not be there)
    pub fn destroy_entity(&self, world_id: WorldID, entity_id: EntityID) -> bool {
        match self.worlds.get(&world_id) {
            Some(entry) => {
                entry.destroy_entity(entity_id);
                true
            }
            None => {
                println!(
                    "Failed to destroy entity {entity_id} due to: Couldn't find World {world_id}!"
                );
                false
            }
        }
    }

    /// Get the the list of current worlds. Note that this is only valid if no stage is being executed, or if called from a Local/Global System, else it might include deleted worlds
    pub fn get_worlds_list(&self) -> Vec<WorldID> {
        let mut worlds: Vec<WorldID> = Vec::new();
        worlds.reserve(self.worlds.len());

        self.worlds.iter().for_each(|map_ref| {
            worlds.push(*map_ref.key());
        });

        worlds
    }

    /// Get the the list of current worlds. Note that this is only valid if no stage is being executed, or if called from a Local/Global System, else it might include deleted worlds
    pub fn get_worlds_list_no_alloc(&self, worlds: &mut Vec<WorldID>) {
        worlds.reserve(self.worlds.len());

        self.worlds.iter().for_each(|map_ref| {
            worlds.push(*map_ref.key());
        });
    }

    // Resets the entity system. That is, destroys all the worlds and creates the default one. DO NOT call this from an world/system update
    fn reset_internal(&self) {
        // Empty commands
        while !self.destroy_world_queue.is_empty() {
            self.destroy_world_queue.pop();
        }

        while !self.merge_worlds_queue.is_empty() {
            self.merge_worlds_queue.pop();
        }

        // Destroy all worlds
        self.worlds.clear();

        // Create default world
        self.create_world_internal(DEFAULT_WORLD); // World 0 is always created

        self.requested_reset.store(false, Ordering::Release);
    }

    // Resets the entity system. That is, destroys all the worlds and creates the default one.
    pub fn reset(&self) {
        let _ = self.requested_reset.compare_exchange_weak(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed,
        );
    }
}

/// The default world that is always created when the entity system starts
/// Note that it might be destroyed
pub const DEFAULT_WORLD: WorldID = 0;

impl EntitySystem {
    fn new() -> Self {
        let new_self = Self {
            pool: ThreadPoolBuilder::new()
                .thread_name(|i| format!("Entity System Thread {i}"))
                .build()
                .expect("Failed to create the entity system thread pool!"),
            delta_time: Default::default(),
            fixed_delta_time: Default::default(),
            requested_reset: Default::default(),
            worlds: Default::default(),
            world_id_counter: Default::default(),
            destroy_world_queue: Default::default(),
            merge_worlds_queue: Default::default(),
        };

        new_self.reset_internal();

        new_self
    }
}

lazy_static! {
    /// Entity System's Worlds
    static ref ENTITY_SYSTEM:  EntitySystem = EntitySystem::new();
}
