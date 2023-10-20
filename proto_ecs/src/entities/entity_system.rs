use std::sync::atomic::{AtomicU16, Ordering};

use lazy_static::lazy_static;
use nohash_hasher::IntSet;

use atomic_float::AtomicF64;

use crate::entities::entity::{EntityID, INVALID_ENTITY_ID};

use crate::core::locking::RwLock;
use crate::local_systems::{StageID, STAGE_COUNT};

use super::{entity::Entity, entity_spawn_desc::EntitySpawnDescription};

/// We just go up. If we ever run out of them we can think of blocks of IDs per thread and a better allocation system
static ENTITY_COUNT: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(INVALID_ENTITY_ID + 1);

/// Allocate a new Entity ID
pub fn allocate_entity_id() -> EntityID {
    // Note: if we ever need to do something more complex with IDs we can do it here

    ENTITY_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Deallocate an Entity ID
pub fn deallocate_entity_id(id: EntityID) {
    assert!(id < ENTITY_COUNT.load(std::sync::atomic::Ordering::Relaxed));

    // Note: if we ever need to do something more complex with IDs we can do it here
}

/// Entity Creation Queue type used by worlds
pub type EntityCreationQueue = scc::Queue<(EntityID, EntitySpawnDescription)>;

/// Entity Map Type that holds all the entities in a World
pub type EntityMap = scc::HashMap<EntityID, Box<RwLock<Entity>>>;

/// World Identifier in the Entity System
pub type WorldID = u16;

#[derive(Debug)]
pub struct World {
    id: WorldID,
    entities: EntityMap,
    creation_queue: EntityCreationQueue,
}

impl World {
    fn new(id: WorldID) -> Self {
        Self {
            id,
            entities: Default::default(),
            creation_queue: Default::default(),
        }
    }
}

/// Entity System map type of Worlds
pub type WorldMap = scc::HashMap<WorldID, World>;

/// Entity System queue type for destroy world commands
pub type WorldDestroyQueue = scc::Queue<WorldID>;

/// Entity System queue type for merge world commands
pub type WorldMergeQueue = scc::Queue<(WorldID, WorldID)>;

#[derive(Debug)]
pub struct EntitySystem {
    delta_time: AtomicF64,
    fixed_delta_time: AtomicF64,
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
    pub fn get_delta_time(&self) -> f64 {
        self.delta_time.load(Ordering::Acquire)
    }

    /// Current unscaled fixed delta time
    #[inline(always)]
    pub fn get_fixed_delta_time(&self) -> f64 {
        self.fixed_delta_time.load(Ordering::Acquire)
    }

    /// Create a new world
    fn create_world_internal(&self, new_id: WorldID) {
        self.worlds
            .insert(new_id, World::new(new_id))
            .expect("Failed to create new world!");
    }

    /// Create a new world and return its world ID
    pub fn create_world(&self) -> WorldID {
        let new_id = self.world_id_counter.fetch_add(1, Ordering::AcqRel) as WorldID;

        self.create_world_internal(new_id);

        new_id
    }

    /// Destroy a world
    fn destroy_world_internal(&self, id: WorldID) {
        if !self.worlds.remove(&id).is_some() {
            println!("Failed to destroy World {id}, maybe it was already destroyed(?)");
        }
    }

    /// Destroy a world and all of its content
    pub fn destroy_world(&self, id: WorldID) {
        self.destroy_world_queue.push(id);
    }

    /// Merge `source` world into the `target` world. This destroys the `source` world
    fn merge_worlds_internal(&self, source: WorldID, target: WorldID) {
        let target_world = self.worlds.get(&target);
        if target_world.is_none() {
            println!(
                "Failed to merge World {source} into World {target} due to missing target world!"
            );
            return;
        }
        let target_world = target_world.unwrap().get_mut();

        let source_world = self.worlds.remove(&source);
        if source_world.is_none() {
            println!(
                "Failed to merge World {source} into World {target} due to missing source world!"
            );
            return;
        }
        let mut source_world = source_world.unwrap().1;

        todo!("Merge source world into target world!");
    }

    /// Merge `source` world into the `target` world. This destroys the `source` world
    pub fn merge_worlds(&self, source: WorldID, target: WorldID) {
        self.merge_worlds_queue.push((source, target));
    }

    /// Process destroy and merge world commands
    fn process_world_command_queues(&self) {
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
        self.process_world_command_queues();
    }

    /// Step the entity system
    pub fn step(&self, new_delta_time: f64) {
        // Set the current unscaled delta time
        self.delta_time.store(new_delta_time, Ordering::Release);

        // Go through all the stages
        for stage_id in 0..STAGE_COUNT {
            self.process_stage(stage_id as StageID);
        }
    }
}

/// The default world that is always created when the entity system starts
/// Note that it might be destroyed
pub const DEFAULT_WORLD: WorldID = 0;

impl EntitySystem {
    fn new() -> Self {
        let new_self = Self {
            delta_time: Default::default(),
            fixed_delta_time: Default::default(),
            worlds: Default::default(),
            world_id_counter: Default::default(),
            destroy_world_queue: Default::default(),
            merge_worlds_queue: Default::default(),
        };

        new_self.create_world_internal(DEFAULT_WORLD); // World 0 is always created

        new_self
    }
}

lazy_static! {
    /// Entity System's Worlds
    static ref ENTITY_SYSTEM:  EntitySystem = EntitySystem::new();
}
