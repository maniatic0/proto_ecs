use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};

use bitvec::store::BitStore;
use lazy_static::lazy_static;

use atomic_float::AtomicF64;

use crate::entities::entity::{EntityID, INVALID_ENTITY_ID};

use super::entity::{self, Entity};
use super::entity_spawn_desc::EntitySpawnDescription;
use crate::core::locking::RwLock;
use crate::entities::entity_allocator::EntityAllocator;
use crate::systems::common::{StageID, STAGE_COUNT};
use crate::systems::global_systems::{GlobalSystem, GlobalSystemID, GlobalSystemRegistry};

use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};

pub use crate::entities::entity_allocator::EntityPtr;

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

/// Queue of global systems used to schedule deletion and creation
pub type GlobalSystemQueue = scc::Queue<GlobalSystemID>;

/// Entity Map Type that holds all the entities in a World
pub type EntityMap = dashmap::DashMap<EntityID, EntityPtr>;

/// Global System Storage inside world
pub type GlobalSystemStorage = RwLock<Box<dyn GlobalSystem>>;

/// Storage for all global systems currently loaded. If not here,
/// it means that it's not loaded
pub type GlobalSystemMap = RwLock<Vec<Option<GlobalSystemStorage>>>;

/// Vector with all the entities inside a world or inside a stage in a world (for faster iteration).
/// Do not use at the same time as the entity map
pub type EntitiesVec = RwLock<Vec<EntityPtr>>;

/// Array used to count how many entities are subscribed to some global system
/// to know when we have to unload them
pub type GlobalSystemCount = Vec<AtomicUsize>;

/// World Identifier in the Entity System
pub type WorldID = u16;

/// A list of global system identifiers, mostly used to
/// Know which global systems should be ran per stage
pub type GlobalSystemIDVec = RwLock<Vec<GlobalSystemID>>;

// A map from global system to the set of entities it has to run
pub type GSEntitiesMap = RwLock<Vec<EntitiesVec>>;

pub type ReparentingQueue = scc::Queue<ReparentingOps>;

/// Possible re-parenting operations
#[derive(Debug)]
enum ReparentingOps {
    SetParent { child: EntityID, parent: EntityID },
    ClearParent(EntityID),
}

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
    reparenting_queue: ReparentingQueue,

    global_system_stages: [GlobalSystemIDVec; STAGE_COUNT],
    global_systems: GlobalSystemMap,
    global_systems_count: GlobalSystemCount,
    gs_creation_queue: GlobalSystemQueue,
    gs_deletion_queue: GlobalSystemQueue,
    /// entities to run per stage per global system
    gs_entity_map: GSEntitiesMap,
}

impl World {
    /// Number of chunks to use for stepping a stage
    /// Maybe this should be variable based on load
    const CHUNKS_NUM: usize = 20;

    pub(crate) fn new(id: WorldID) -> Self {
        let gs_count = GlobalSystemRegistry::get_global_registry()
            .read()
            .get_global_system_count();
        let mut gs_count_array = Vec::new();
        let mut gs_entity_map: Vec<EntitiesVec> = Vec::with_capacity(gs_count);
        let mut gs_map = Vec::with_capacity(gs_count);

        for _ in 0..gs_count {
            gs_count_array.push(AtomicUsize::ZERO);
            gs_entity_map.push(EntitiesVec::default());
            gs_map.push(None);
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
            reparenting_queue: Default::default(),
            global_systems: GlobalSystemMap::new(gs_map),
            global_systems_count: gs_count_array,
            global_system_stages: core::array::from_fn(|_| Default::default()),
            gs_creation_queue: Default::default(),
            gs_deletion_queue: Default::default(),
            gs_entity_map: RwLock::new(gs_entity_map),
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
        if cfg!(debug_assertions) {
            // Check that the spawn desc makes sense. Maybe change the cfg macro to be separate of all debug assertions
            spawn_desc.check_panic();
        }
        let new_id = allocate_entity_id();
        self.creation_queue
            .push(RwLock::new(Some((new_id, spawn_desc))));
        new_id
    }

    /// Create a new entity based on its spawn description
    fn create_entity_internal(&self, id: EntityID, spawn_desc: EntitySpawnDescription) {
        // Allocate entity from the global allocator
        let global_allocator = EntityAllocator::get_global();
        let mut entity_ptr = global_allocator.write().allocate();
        entity_ptr.init(id, spawn_desc);

        let old = self.entities.insert(id, entity_ptr);
        assert!(
            old.is_none(),
            "Duplicated Entity ID, old entity {:?}",
            old.unwrap()
        );

        // Insert entity for iteration
        {
            let mut entities_all = self.entities_all.write();
            entities_all.push(entity_ptr);
        }

        let entity_ref = unsafe { &*(*entity_ptr).data_ptr() };

        // Schedule this entity to run in the right stage
        for (stage_id, stage_vec) in self.entities_stages.iter().enumerate() {
            let stage_id = stage_id as StageID;
            if entity_ref.should_run_in_stage(stage_id) {
                stage_vec.write().push(entity_ptr);
            }
        }

        // Initialize every global system that is not currently loaded
        for &gs_id in entity_ref.get_global_systems() {
            {
                let gs_count = &self.global_systems_count;
                gs_count[gs_id as usize].fetch_add(1, Ordering::Relaxed);
            }

            if !self.global_system_is_loaded(gs_id) {
                self.gs_creation_queue.push(gs_id);
            }

            // Add this entity to the entity vector for each GS it requires
            let mut entities_per_gs = self.gs_entity_map.write();
            let gs_entities = &mut entities_per_gs[gs_id as usize];
            gs_entities.write().push(entity_ptr);
        }
    }

    /// Destroy an entity. Note that the entity will be destroyed at the end of the current stage
    pub fn destroy_entity(&self, id: EntityID) {
        self.deletion_queue.push(id);
    }

    /// Destroy an entity
    pub fn destroy_entity_internal(&self, id: EntityID) {
        // Before deleting an entity, we have to check if the entity
        let prev = self.entities.remove(&id);
        if prev.is_none() {
            println!("Failed to destroy Entity {id}, maybe it was already deleted (?)");
            return;
        }
        let (_id, entity_ptr) = prev.unwrap();

        // TODO I'm not sure this implementation is the best option for recursive deletion.

        // Delete all your children bellow you if you're a spatial entity
        if entity_ptr.is_live() && entity_ptr.read().is_spatial_entity()
        // might be deleted
        {
            // Note that the only parent that should be deleted with `clear_parent`
            // is the first entity to be deleted in the hierarchy, for the rest we can just forget
            // about their transform state since it doesn't matter after deletion.
            // ? Can this be a problem if we want entities to have a `on_delete` callback? do we want one?

            Entity::clear_parent(entity_ptr);
            let mut entity_stack = Vec::with_capacity(100);
            let mut ids_to_delete = Vec::with_capacity(100);
            entity_stack.push(entity_ptr);

            // Collect all entities in the hierarchy and delete their transform
            while !entity_stack.is_empty() {
                let next_entity_ptr = entity_stack.pop().unwrap();
                let mut entity = next_entity_ptr.write();

                let entity_transform = entity.get_transform().unwrap();
                for entity_ptr in entity_transform.children.iter() {
                    entity_stack.push(entity_ptr.clone());
                }

                entity.delete_transform();
                ids_to_delete.push(entity.get_id());
            }

            // delete all entities in the hierarchy. The order doesn't matter,
            // so this might be a good place to add parallel execution with rayon
            for id in ids_to_delete {
                self.destroy_entity_internal(id);
            }

            // TODO we have to update the list of entities to run per stage after all children were deleted
        }

        // Destroy entity from iteration lists
        {
            let mut entities_all = self.entities_all.write();
            for i in 0..entities_all.len() {
                let vec_ref = entities_all[i];
                if vec_ref == entity_ptr {
                    entities_all.swap_remove(i);
                    break;
                }
            }
        }

        // Decrease counters for global systems in this entity
        {
            let gs_counts = &self.global_systems_count;
            for &gs_id in entity_ptr.read().get_global_systems() {
                let result = gs_counts[gs_id as usize].fetch_sub(1, Ordering::Relaxed);

                if result == 1 {
                    // this was the last entity requiring this GS
                    self.gs_deletion_queue.push(gs_id);
                }

                // Delete this entity from the GS entity vec
                let gs_entities_map = self.gs_entity_map.read();
                let gs_entities = &mut gs_entities_map[gs_id as usize].write();

                for i in 0..gs_entities.len() {
                    let other_entity_ptr = gs_entities[i];
                    if other_entity_ptr == entity_ptr {
                        gs_entities.swap_remove(i);
                    }
                }
            }
        }

        for (stage_id, stage_vec) in self.entities_stages.iter().enumerate() {
            let stage_id = stage_id as StageID;
            if entity_ptr.read().should_run_in_stage(stage_id) {
                let mut stage_vec = stage_vec.write();
                for (index, &vec_ref) in stage_vec.iter().enumerate() {
                    if vec_ref == entity_ptr {
                        stage_vec.swap_remove(index);
                        break;
                    }
                }
            }
        }

        deallocate_entity_id(id);
        // Actually destroy entity
        let global_allocator = EntityAllocator::get_global();
        global_allocator.write().free(&entity_ptr);
    }

    /// Request to make `parent_id` the parent of `entity_id`.
    ///
    /// The reparenting operation will take effect the next frame, not the current frame.
    /// You can call this over an entity that will be created for the next frame
    pub fn set_entity_parent(&self, entity_id: EntityID, parent_id: EntityID) {
        self.reparenting_queue.push(ReparentingOps::SetParent {
            child: entity_id,
            parent: parent_id,
        });
    }

    /// Request to clear the parent of `entity_id`.
    ///
    /// The reparenting operation will take effect the next frame, not the current frame.
    pub fn clear_entity_parent(&self, entity_id: EntityID) {
        self.reparenting_queue
            .push(ReparentingOps::ClearParent(entity_id));
    }

    pub(super) fn set_entity_parent_internal(&self, entity_id: EntityID, parent_id: EntityID) {
        let mut old_stages_to_run = [false; STAGE_COUNT];
        let parent_ptr = self
            .entities
            .get(&parent_id)
            .expect("Entity should be created by now!");
        let entity_ptr = self
            .entities
            .get(&entity_id)
            .expect("Entity should be created by now!");

        for stage_id in 0..STAGE_COUNT {
            old_stages_to_run[stage_id] =
                parent_ptr.read().should_run_in_stage(stage_id as StageID);
        }

        Entity::set_parent(*entity_ptr, *parent_ptr);

        // Now check if we have to update the internal local system running list
        let mut root = *parent_ptr;

        // Get hierarchy root
        while !root.read().is_root() {
            let parent;
            {
                let root_obj = root.read();
                let root_transform = root_obj.get_transform().unwrap();
                parent = root_transform.parent.unwrap();
            }
            root = parent;
        }

        // TODO if the entity didn't had a parent, it might be a root that should be removed from the per-stage run list
        for (stage_id, stage_vec) in self.entities_stages.iter().enumerate() {
            if root.read().should_run_in_stage(stage_id as StageID) && !old_stages_to_run[stage_id]
            {
                stage_vec.write().push(root);
            }
        }
    }

    pub(super) fn clear_parent_internal(&self, entity_id: EntityID) {
        // TODO We have to:
        //  Actually clear the parent
        //  remove the root of this entity from the per-stage entity list
        //  add this entity to the per-stage entity list
        todo!("Implement this like the re-parent")
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

    fn process_global_systems_commands(&self) {
        let mut changed = false;
        // Delete global systems scheduled for deletion
        while let Some(val) = self.gs_deletion_queue.pop() {
            let gs_to_delete = **val;

            // If just deleted skip deletion
            if !self.global_system_is_loaded(gs_to_delete) {
                self.unload_global_system(gs_to_delete);
                changed = true;
            }
        }

        // Create global systems scheduled for creation
        while let Some(val) = self.gs_creation_queue.pop() {
            let gs_to_create = **val;

            // If already created just skip creation
            if !self.global_system_is_loaded(gs_to_create) {
                self.load_global_system(gs_to_create);
                changed = true;
            }
        }

        // we have to sort stage vectors so that global systems run in the right order
        if changed {
            for stage_vec_lock in self.global_system_stages.iter() {
                let mut stage_vec = stage_vec_lock.write();
                stage_vec.sort();
            }
        }
    }

    /// Process all entity commands
    fn process_entity_commands(&self) {
        // Process all deletions
        if !self.deletion_queue.is_empty() {
            let mut work: Vec<EntityID> = Vec::new();
            while let Some(val) = self.deletion_queue.pop() {
                work.push(**val);
            }

            work.into_par_iter().for_each(|id| {
                self.destroy_entity_internal(id);
            });
        }

        // Process all creations
        if !self.creation_queue.is_empty() {
            let mut work: Vec<(EntityID, EntitySpawnDescription)> = Vec::new();
            while let Some(val) = self.creation_queue.pop() {
                work.push(val.write().take().unwrap())
            }

            work.into_par_iter().for_each(|(id, spawn_desc)| {
                self.create_entity_internal(id, spawn_desc);
            });
        }

        // Process re-parenting
        if !self.reparenting_queue.is_empty() {
            // No parallelism allowed here, reparenting operations
            // require careful manipulation between references of entities
            while let Some(op) = self.reparenting_queue.pop() {
                match **op {
                    ReparentingOps::SetParent { child, parent } => {
                        self.set_entity_parent_internal(child, parent)
                    }
                    ReparentingOps::ClearParent(entity) => self.clear_parent_internal(entity),
                }
            }
        }
    }

    /// Process a stage in this world
    fn run_stage(&self, stage_id: StageID) {
        // Process all the entity and global systems commands before the stage
        self.process_entity_commands();
        self.process_global_systems_commands();

        {
            // Run Stage in all entities
            let entities_stage = self.entities_stages[stage_id as usize].read();
            if entities_stage.is_empty() {
                // Nothing to do, no more commands can be created
                // TODO: Check this for Global Systems. They might need to execute?
                return;
            }

            println!("Stage has {} entities", entities_stage.len());
            entities_stage
                .par_chunks(World::CHUNKS_NUM)
                .for_each(|map_refs| {
                    for map_ref in map_refs {
                        // Note we don't need to take the lock as we are 100% sure rayon is executing disjoint tasks.
                        let entity = unsafe { &mut *map_ref.data_ptr() };
                        let mut recursion_stack = Vec::with_capacity(20);

                        println!("Entity is {}", entity.get_name());

                        // Check if stage is enabled before running
                        if !entity.is_spatial_entity() && entity.is_stage_enabled(stage_id) {
                            // If not a spatial entity, just run it
                            entity.run_stage(self, stage_id);
                        } else if entity.is_spatial_entity() && entity.should_run_in_stage(stage_id)
                        {
                            // If a spatial entity, run recursively
                            entity.run_stage_recursive_no_alloc(
                                self,
                                stage_id,
                                &mut recursion_stack,
                            );
                        }
                    }
                });
        }

        // Run all global systems
        {
            let gs_stage = self.global_system_stages[stage_id as usize].read();
            let gs_registry = GlobalSystemRegistry::get_global_registry().read();
            let gs_storages = self.global_systems.read();
            for &gs_id in gs_stage.iter() {
                let entry = gs_registry.get_entry_by_id(gs_id);
                let mut storage = gs_storages[gs_id as usize].as_ref().unwrap().write();
                let current_fn = entry.functions[stage_id as usize]
                    .expect("This global system should have a function for the current stage");

                let mut stage_entities = self.gs_entity_map.write();
                let current_stage_entities = &mut stage_entities[gs_id as usize];

                (current_fn)(&mut storage, self, &self.entities, &current_stage_entities);
            }
        }

        // Process all the entity commands created in the stage
        self.process_entity_commands();
        self.process_global_systems_commands();
    }

    fn global_system_is_loaded(&self, global_system_id: GlobalSystemID) -> bool {
        self.global_systems.read()[global_system_id as usize].is_some()
    }

    /// Creates and initializes a new global system.
    /// After adding a new global systems the list of global systems to
    /// run per stage will be out of order. You should sort those lists after
    /// adding more global systems.
    fn load_global_system(&self, global_system_id: GlobalSystemID) {
        debug_assert!(
            self.global_systems.read()[global_system_id as usize].is_none(),
            "Global system was already loaded"
        );

        let gs_registry = GlobalSystemRegistry::get_global_registry().read();

        let gs = gs_registry.create_by_id(global_system_id);
        self.global_systems.write()[global_system_id as usize] = Some(RwLock::new(gs));

        // Add this global system to the list of global systems to run per stage
        // to its corresponding stages
        let entry = gs_registry.get_entry_by_id(global_system_id);
        for (i, stage_fn) in entry.functions.iter().enumerate() {
            if stage_fn.is_none() {
                continue;
            }

            self.global_system_stages[i].write().push(global_system_id);
        }
    }

    /// Deletes a global system.
    /// After deleting a global system the list of global systems to
    /// run per stage will be out of order. You should sort those lists after
    /// unloading global systems.
    fn unload_global_system(&self, global_system_id: GlobalSystemID) {
        debug_assert!(
            self.global_systems.read()[global_system_id as usize].is_some(),
            "Global system was already unloaded"
        );

        self.global_systems.write()[global_system_id as usize] = None;

        // Remove this global system from the stages that require their functions
        let registry = GlobalSystemRegistry::get_global_registry().read();
        let entry = registry.get_entry_by_id(global_system_id);
        for (i, gs_fn) in entry.functions.iter().enumerate() {
            if gs_fn.is_none() {
                continue;
            }
            let mut stage_gs = self.global_system_stages[i].write();
            let gs_index = *stage_gs
                .iter()
                .find(|&&gs_id| gs_id == global_system_id)
                .expect("Programming Error: This global system should be in the stages vector");

            stage_gs.swap_remove(gs_index as usize);
        }
    }

    /// Merge target world into this world
    fn merge_world(&mut self, mut _target: Self) {
        todo!("Implement world merge!")
    }

    /// Get a reference to the entity map.
    ///
    /// This function is intended to be used for tests
    /// to check the state of some entity.
    ///
    /// DO NOT USE THIS FUNCTION OUTSIDE TESTS
    #[inline(always)]
    #[allow(unused)]
    pub(super) fn get_entities(&self) -> &EntityMap {
        &self.entities
    }

    /// Get a reference to the global system map.
    ///
    /// This function is intended to be used for tests
    /// to check the state of some global system.
    ///
    /// DO NOT USE THIS FUNCTION OUTSIDE TESTS
    #[inline(always)]
    #[allow(unused)]
    pub(super) fn get_global_systems(&self) -> &GlobalSystemMap {
        &self.global_systems
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
/// Errors produced by the Entity System
pub enum EntitySystemError {
    /// Failed to find the specified world
    WorldNotFound,
}

impl std::fmt::Display for EntitySystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntitySystemError::WorldNotFound => write!(f, "World Not Found"),
        }
    }
}

impl std::error::Error for EntitySystemError {}

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

    /// Create a new entity in World `world_id` based on its spawn description. Note that the entity will spawn at the end of the current stage. If the world cannot be found, it returns an err
    pub fn create_entity(
        &self,
        world_id: WorldID,
        spawn_desc: EntitySpawnDescription,
    ) -> Result<EntityID, EntitySystemError> {
        match self.worlds.get(&world_id) {
            Some(entry) => Ok(entry.create_entity(spawn_desc)),
            None => {
                println!("Failed to create entity due to: Couldn't find World {world_id}!");
                Err(EntitySystemError::WorldNotFound)
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
        let mut worlds: Vec<WorldID> = Vec::with_capacity(self.worlds.len());

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

    /// Get a reference to worlds.
    ///
    /// This function is intended to be used for tests
    /// to check the state of some world.
    ///
    /// DO NOT USE THIS FUNCTION OUTSIDE TESTS
    #[inline(always)]
    #[allow(unused)]
    pub(super) fn get_worlds(&self) -> &WorldMap {
        return &self.worlds;
    }

    /// Get a reference to an entity from the specified world.
    ///
    /// This function is intended to be used in tests only.
    /// DO NOT USE THIS FUNCTION OUTSIDE TESTS
    #[cfg(test)]
    pub(super) fn get_entity(&self, world_id: WorldID, entity_id: EntityID) -> EntityPtr {
        let world = self.worlds.get(&world_id).unwrap();
        let entity = world.entities.get(&entity_id).unwrap();
        return entity.clone();
    }

    /// Get a reference to the worldmap
    ///
    /// This function is intended to be used in tests only.
    /// DO NOT USE THIS FUNCTION OUTSIDE TESTS
    #[cfg(test)]
    pub(super) fn get_world_map(&self) -> &WorldMap {
        return &self.worlds;
    }

    /// Run a step for the specified world. Specially useful to run a word per test
    /// avoid triggering concurrent steps for all stored worlds
    #[cfg(test)]
    pub(super) fn step_world(
        &self,
        new_delta_time: DeltaTimeType,
        fixed_delta_time: DeltaTimeType,
        world_id: WorldID,
    ) {
        // Set the current unscaled delta time
        self.delta_time.store(new_delta_time, Ordering::Release);
        self.fixed_delta_time
            .store(fixed_delta_time, Ordering::Release);

        self.worlds
            .get(&world_id)
            .and_then(|world| {
                world
                    .update_delta_time_internal(self.get_delta_time(), self.get_fixed_delta_time());
                Some(())
            })
            .expect("World not found");

        for stage_id in 0..STAGE_COUNT {
            self.process_stage_world(stage_id as StageID, world_id)
        }
    }

    /// Process a stage for a specific world.
    ///
    /// This functions is intended to be used in tests only.
    /// DO NOT USE OUTSIDE A TEST
    #[cfg(test)]
    pub(super) fn process_stage_world(&self, stage_id: StageID, world_id: WorldID) {
        // Process all commands created before the stage
        self.process_world_command_queues();

        // Process worlds in parallel
        self.worlds
            .get(&world_id)
            .and_then(|world| {
                world.run_stage(stage_id);
                Some(())
            })
            .expect("World should exists by now");

        // Process all commands created in the stage
        self.process_world_command_queues();
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
            world_id_counter: AtomicU16::new(DEFAULT_WORLD + 1), // Note that the default world has id 0
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
