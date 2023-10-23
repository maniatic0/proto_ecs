use std::sync::atomic::{AtomicU16, Ordering};

use lazy_static::lazy_static;

use atomic_float::AtomicF64;

use crate::entities::entity::{EntityID, INVALID_ENTITY_ID};

use crate::core::locking::{RwLock, SyncUnsafeCell};
use crate::systems::common::{StageID, STAGE_COUNT};

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

/// Entity Map Type that holds all the entities in a World
pub type EntityMap = scc::HashMap<EntityID, Box<Entity>>;

/// World Identifier in the Entity System
pub type WorldID = u16;

pub struct World {
    id: WorldID,
    entities: EntityMap,
    entities_work: SyncUnsafeCell<Vec<EntityID>>,
    creation_queue: EntityCreationQueue,
    deletion_queue: EntityDeletionQueue,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("id", &self.id)
            .field("entities", &self.entities)
            .field("creation_queue", &self.creation_queue)
            .field("deletion_queue", &self.deletion_queue)
            .finish()
    }
}

impl World {
    pub(crate) fn new(id: WorldID) -> Self {
        Self {
            id,
            entities: Default::default(),
            entities_work: Default::default(),
            creation_queue: Default::default(),
            deletion_queue: Default::default(),
        }
    }

    pub fn get_id(&self) -> WorldID {
        self.id
    }

    /// Create a new entity based on its spawn description. Note that the entity will spawn at the end of the current stage
    pub fn create_entity(&self, spawn_desc: EntitySpawnDescription) -> EntityID {
        let new_id = allocate_entity_id();
        self.creation_queue
            .push(RwLock::new(Some((new_id.clone(), spawn_desc))));
        new_id
    }

    /// Create a new entity based on its spawn description
    fn create_entity_internal(&self, id: EntityID, spawn_desc: EntitySpawnDescription) {
        self.entities
            .insert(id, Box::new(Entity::init(id, spawn_desc)))
            .expect("Failed to create entity!");
    }

    /// Destroy an entity. Note that the entity will be destroyed at the end of the current stage
    pub fn destroy_entity(&self, id: EntityID) {
        self.deletion_queue.push(id);
    }

    /// Destroy an entity
    pub fn destroy_entity_internal(&self, id: EntityID) {
        if self.entities.remove(&id).is_none() {
            println!("Failed to destroy Entity {id}, maybe it was already deleted (?)");
        }
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
        let entities_work = unsafe { &mut *self.entities_work.get() };
        entities_work.clear(); // Clear old stuff
        self.entities.scan(|id, _| {
            entities_work.push(*id);
        });

        entities_work.par_iter().for_each(|id| {
            let mut binding = self.entities.get(&id).unwrap();
            let entity = binding.get_mut();

            // Check if stage is enabled
            if !entity.is_stage_enabled(stage_id) {
                return;
            }

            entity.run_stage(&self, stage_id);
        });

        // Process all the entity commands created in the stage
        self.process_entity_commands();
    }

    /// Merge target world into this world
    fn merge_world(&mut self, mut target: Self) {
        todo!("Implement world merge!")
    }
}

/// Entity System map type of Worlds
pub type WorldMap = scc::HashMap<WorldID, World>;

/// Entity System queue type for destroy world commands
pub type WorldDestroyQueue = scc::Queue<WorldID>;

/// Entity System queue type for merge world commands
pub type WorldMergeQueue = scc::Queue<(WorldID, WorldID)>;

pub struct EntitySystem {
    pool: ThreadPool,
    delta_time: AtomicF64,
    fixed_delta_time: AtomicF64,
    worlds: WorldMap,
    worlds_work: SyncUnsafeCell<Vec<WorldID>>,
    world_id_counter: AtomicU16,
    destroy_world_queue: WorldDestroyQueue,
    merge_worlds_queue: WorldMergeQueue,
}

impl std::fmt::Debug for EntitySystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntitySystem")
            .field("pool", &self.pool)
            .field("delta_time", &self.delta_time)
            .field("fixed_delta_time", &self.fixed_delta_time)
            .field("worlds", &self.worlds)
            .field("world_id_counter", &self.world_id_counter)
            .field("destroy_world_queue", &self.destroy_world_queue)
            .field("merge_worlds_queue", &self.merge_worlds_queue)
            .finish()
    }
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
        let target_world = self.worlds.get(&target);
        if target_world.is_none() {
            println!(
                "Failed to merge World {source} into World {target} due to missing target world!"
            );
            return;
        }
        let mut binding = target_world.unwrap();
        let target_world = binding.get_mut();

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
        if !self.worlds.is_empty() {
            let worlds_work = unsafe { &mut *self.worlds_work.get() };
            worlds_work.clear(); // Clear old run
            worlds_work.reserve_exact(self.worlds.len());
            self.worlds.scan(|world_id, _| {
                worlds_work.push(*world_id);
            });

            self.pool.install(|| {
                worlds_work.par_iter().for_each(|world_id| {
                    self.worlds
                        .get(&world_id)
                        .unwrap()
                        .get()
                        .run_stage(stage_id);
                });
            });
        }

        // Process all commands created in the stage
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

    /// Create a new entity in World `world_id` based on its spawn description. Note that the entity will spawn at the end of the current stage. If the world cannot be found, it returns the spawn desc as an err
    pub fn create_entity(
        &self,
        world_id: WorldID,
        spawn_desc: EntitySpawnDescription,
    ) -> Result<EntityID, EntitySpawnDescription> {
        match self.worlds.get(&world_id) {
            Some(entry) => Ok(entry.get().create_entity(spawn_desc)),
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
                entry.get().destroy_entity(entity_id);
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

        self.worlds.scan(|world_id, _| {
            worlds.push(*world_id);
        });

        worlds
    }

    /// Get the the list of current worlds. Note that this is only valid if no stage is being executed, or if called from a Local/Global System, else it might include deleted worlds
    pub fn get_worlds_list_no_alloc(&self, worlds: &mut Vec<WorldID>) {
        worlds.reserve(self.worlds.len());

        self.worlds.scan(|world_id, _| {
            worlds.push(*world_id);
        });
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
            worlds: Default::default(),
            worlds_work: Default::default(),
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
