use std::sync::atomic::Ordering;

use crate::{
    core::{
        casting::{cast, cast_mut, CanCast},
        ids::IDLocator,
    },
    data_group::{DataGroup, DataGroupID, DataGroupInitType, DataGroupRegistry},
    entities::entity_spawn_desc::EntitySpawnDescription,
    get_id,
    systems::common::Dependency,
    systems::{
        global_systems::{GlobalSystemDesc, GlobalSystemID},
        local_systems::{LocalSystemDesc, LocalSystemRegistry},
    },
};
use proto_ecs::systems::common::{StageID, STAGE_COUNT};
use proto_ecs::systems::local_systems::{SystemClassID, SystemFn};

use bitvec::prelude::{BitArr, BitArray};
use nohash_hasher::{IntMap, IntSet};
use vector_map::{set::VecSet, VecMap};

use super::{entity_system::{World, EntityPtr}, transform_datagroup::Transform};

pub type EntityID = u64;

/// The invalid entity ID
pub const INVALID_ENTITY_ID: EntityID = 0;

/// Map type used by entities to store datagroups
pub type DataGroupVec = Vec<Box<dyn DataGroup>>;

/// Type for use when indexing datagroups in entities
/// It defines the max number of them in an entity
pub type DataGroupIndexingType = u16;

/// This is the index considered invalid
pub const INVALID_DATAGROUP_INDEX: DataGroupIndexingType = DataGroupIndexingType::MAX;

/// Max number of datagroups that can be in an entity
pub const MAX_DATAGROUP_LEN: DataGroupIndexingType = INVALID_DATAGROUP_INDEX;

/// Map type used by entities to store what local systems it has
pub type LocalSystemMap = IntSet<SystemClassID>;

/// Map type used by entities to store local systems' enabled stages
pub type StageEnabledMap = BitArr!(for STAGE_COUNT);

/// From where to get the local system datagroup indices
type LocalSystemIndexingVec = Vec<DataGroupIndexingType>;

/// Map type used by entities to store local systems' execution functions per stage
pub type StageMap = VecMap<StageID, Vec<(DataGroupIndexingType, SystemFn)>>;

/// Map type used by entities to store the reference to its children
pub type ChildrenMap = VecSet<EntityID>;

#[derive(Default)]
pub struct Entity {
    id: EntityID,
    name: String,
    debug_info: String,

    datagroups: DataGroupVec,

    local_systems_indices: LocalSystemIndexingVec,
    local_systems_map: LocalSystemMap,
    ls_stage_enabled_map: StageEnabledMap,
    stage_map: StageMap,

    global_systems: IntSet<GlobalSystemID>,

    // Index of the transform datagroup in the `datagroups` vector
    transform_index: DataGroupIndexingType
}

impl Entity {
    pub(super) fn init(id: EntityID, spawn_desc: EntitySpawnDescription) -> Self {
        let EntitySpawnDescription {
            name,
            debug_info,
            data_groups,
            local_systems,
            global_systems,
        } = spawn_desc;

        // Init Datagroups
        let dg_registry = DataGroupRegistry::get_global_registry().read();
        let mut datagroups = DataGroupVec::new();

        let mut transform_requested = false;
        for (id, init_params) in data_groups {
            let entry = dg_registry.get_entry_by_id(id);
            
            let mut new_dg = (entry.factory_func)();

            match init_params {
                DataGroupInitType::Uninitialized(msg) => {
                    panic!("Uninitialized DataGroup '{}': {msg}", entry.name);
                }
                DataGroupInitType::NoInit => (),
                DataGroupInitType::NoArg => new_dg.__init__(None),
                DataGroupInitType::Arg(param) => new_dg.__init__(Some(param)),
                DataGroupInitType::OptionalArg(param) => new_dg.__init__(param),
            }

            datagroups.push(new_dg);

            transform_requested = transform_requested || id == Transform::get_id();
        }
        assert!(datagroups.len() <= MAX_DATAGROUP_LEN as usize);

        // Sort them to be able to use binary search
        datagroups.sort_by_key(|dg| dg.get_id());
        let mut transform_index = INVALID_DATAGROUP_INDEX;
        if transform_requested
        {
            transform_index = datagroups
                .binary_search_by_key(
                    &Transform::get_id(), 
                    |dg|{dg.get_id()}
                ).unwrap() as DataGroupIndexingType;
        }

        // Build temp map for their positions (for Local Systems lookup)
        let mut dg_to_pos_map: IntMap<DataGroupID, DataGroupIndexingType> = IntMap::default();
        for (pos, dg_id) in datagroups.iter().enumerate() {
            dg_to_pos_map.insert(dg_id.get_id(), pos as DataGroupIndexingType);
        }

        // Build stage information and collect datagroup indices
        let mut ls_stage_enabled_map = BitArray::ZERO;
        let mut stage_map = StageMap::new();
        let mut local_systems_indices: LocalSystemIndexingVec = Vec::new();

        let mut sorted_local_systems: Vec<SystemClassID> = local_systems.iter().copied().collect();
        sorted_local_systems.sort();

        let ls_registry = LocalSystemRegistry::get_global_registry().read();
        for &id in &sorted_local_systems {
            let entry = ls_registry.get_entry_by_id(id);

            entry
                .functions
                .iter()
                .enumerate()
                .for_each(|(stage_id, fun)| {
                    let stage_id = stage_id as StageID;
                    match fun {
                        None => (),
                        Some(fun) => {

                            // Mark this stage as enabled if there's a function for it
                            if !ls_stage_enabled_map[stage_id as usize] {
                                ls_stage_enabled_map.set(stage_id as usize, true);
                                stage_map.insert(stage_id, Vec::new());
                            }

                            local_systems_indices.reserve_exact(entry.dependencies.len());

                            for dep in &entry.dependencies {
                                match dep {
                                    Dependency::DataGroup(dg_id) => local_systems_indices.push(
                                        *dg_to_pos_map.get(dg_id).expect(
                                            "Local System is missing datagroup dependency!",
                                        ),
                                    ),
                                    Dependency::OptionalDG(dg_id) => match dg_to_pos_map.get(dg_id)
                                    {
                                        Some(pos) => local_systems_indices.push(*pos),
                                        None => local_systems_indices.push(INVALID_DATAGROUP_INDEX),
                                    },
                                }
                            }

                            let stage = stage_map.get_mut(&stage_id).unwrap();
                            stage.push((entry.dependencies.len() as DataGroupIndexingType, *fun));
                        }
                    }
                });
        }
        local_systems_indices.shrink_to_fit();

        let mut entity = Self {
            id,
            name,
            debug_info,
            datagroups,
            local_systems_indices,
            local_systems_map: local_systems,
            ls_stage_enabled_map,
            stage_map,
            global_systems,
            transform_index
        };

        // Remember to initialize transform 
        if entity.is_spatial_entity()
        {
            entity.init_transform();
        }

        entity
    }

    #[inline(always)]
    pub fn get_id(&self) -> EntityID {
        self.id
    }

    #[inline(always)]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    #[inline(always)]
    pub fn get_debug_info(&self) -> &str {
        &self.debug_info
    }

    #[inline(always)]
    pub fn get_datagroups(&self) -> &DataGroupVec {
        &self.datagroups
    }

    #[inline]
    pub fn get_datagroup_by_id(&self, id: DataGroupID) -> Option<&dyn DataGroup> {
        let pos = self.datagroups.binary_search_by_key(&id, |dg| dg.get_id());
        match pos {
            Ok(pos) => Some(self.datagroups[pos].as_ref()),
            Err(_) => None,
        }
    }

    #[inline]
    pub fn get_datagroup_by_id_mut(&mut self, id: DataGroupID) -> Option<&mut dyn DataGroup> {
        let pos = self.datagroups.binary_search_by_key(&id, |dg| dg.get_id());
        match pos {
            Ok(pos) => Some(self.datagroups[pos].as_mut()),
            Err(_) => None,
        }
    }

    #[inline(always)]
    pub fn get_datagroup<DG>(&self) -> Option<&DG>
    where
        DG: IDLocator + DataGroup + CanCast + Sized + 'static,
    {
        self.get_datagroup_by_id(get_id!(DG)).map(|dg| cast(dg))
    }

    #[inline(always)]
    pub fn get_datagroup_mut<DG>(&mut self) -> Option<&mut DG>
    where
        DG: IDLocator + DataGroup + CanCast + Sized + 'static,
    {
        self.get_datagroup_by_id_mut(get_id!(DG)).map(|dg| cast_mut(dg))
    }

    /// Use this function to mark this as without transform. 
    /// 
    /// Useful when you want an entity to forget about its transform.
    pub(super) fn delete_transform(&mut self)
    {
        // the transform can't actually be deleted, 
        // we just set its index to an invalid value so that all
        // `get_transform` operations return `None`
        self.transform_index = INVALID_DATAGROUP_INDEX;
    }

    #[inline(always)]
    pub fn get_transform(&self) -> Option<&Transform>
    {
        if self.transform_index == INVALID_DATAGROUP_INDEX
        { None }
        else
        { Some(cast(&self.datagroups[self.transform_index as usize])) }
    }

    #[inline(always)]
    pub fn get_transform_mut(&mut self) -> Option<&mut Transform>
    {
        if self.transform_index == INVALID_DATAGROUP_INDEX
        { None }
        else
        { Some(cast_mut(&mut self.datagroups[self.transform_index as usize])) }
    }

    #[inline(always)]
    pub fn get_local_systems(&self) -> &LocalSystemMap {
        &self.local_systems_map
    }

    #[inline(always)]
    pub fn get_global_systems(&self) -> &IntSet<GlobalSystemID> {
        &self.global_systems
    }

    #[inline(always)]
    pub fn contains_local_system_by_id(&self, id: SystemClassID) -> bool {
        self.get_local_systems().contains(&id)
    }

    #[inline(always)]
    pub fn contains_local_system<S>(&self) -> bool
    where
        S: IDLocator + LocalSystemDesc,
    {
        self.contains_local_system_by_id(get_id!(S))
    }

    #[inline(always)]
    pub fn contains_global_system_by_id(&self, id: GlobalSystemID) -> bool {
        self.get_global_systems().contains(&id)
    }

    #[inline(always)]
    pub fn contains_global_system<S>(&self) -> bool
    where
        S: IDLocator + GlobalSystemDesc,
    {
        self.contains_global_system_by_id(get_id!(S))
    }

    #[inline(always)]
    pub fn get_ls_stage_enabled_map(&self) -> &StageEnabledMap {
        &self.ls_stage_enabled_map
    }

    #[inline(always)]
    /// If a stage is enabled for this entity
    pub fn is_stage_enabled(&self, stage_id: StageID) -> bool {

        self.ls_stage_enabled_map[stage_id as usize]
    }

    /// Checks if this entity should be scheduled to run in the specified stage. 
    /// 
    /// Spatial entities that are not root entities are not scheduled to be ran
    /// by the engine, their parent should run them instead
    /// 
    /// Note: this function is used by the engine to check if this entity 
    /// should be included in the list of entities to run per stage
    pub(super) fn should_run_in_stage(&self, stage_id: StageID) -> bool
    {
        if !self.is_spatial_entity()
        {
            return self.ls_stage_enabled_map[stage_id as usize];
        }
        else if self.is_root() {
            let hierarchy = *self.get_transform().as_ref().unwrap();
            let count_for_stage = hierarchy.stage_count[stage_id as usize].load(Ordering::Acquire);
            return count_for_stage > 0;
        }

        // Never add spatial-non-root entities to the stage list, 
        return false;
    }


    /// Runs a stage. Note that it panics if the stage is not enabled
    /// Only to be called by the entity system
    pub(super) fn run_stage(&mut self, world: &World, stage_id: StageID) {
        debug_assert!(
            self.is_stage_enabled(stage_id),
            "Check if the stage is enabled before running it!"
        );

        let stage = self
            .stage_map
            .get_mut(&stage_id)
            .expect("Uninitialized Entity or Entity in undefined state!");

        let mut indices_start: usize = 0;

        for (indices_num, local_sys_fun) in stage {
            let indices_num = *indices_num as usize;
            (local_sys_fun)(
                world,
                self.id,
                &self.local_systems_indices[indices_start..(indices_start + indices_num)],
                &mut self.datagroups,
            );
            indices_start += indices_num;
        }
    }

    pub(super) fn run_stage_recursive(&mut self, world: &World, stage_id: StageID)
    {
        debug_assert!(self.is_spatial_entity(), "Can't recursively run stages for a non-spatial entity");
        let mut recursion_stack : Vec<EntityPtr> = Vec::with_capacity(20);
        self.run_stage_recursive_no_alloc(world, stage_id, &mut recursion_stack);
    }

    /// Run a stage recursively for an entity which is a spatial entity.
    /// 
    /// This function will ensure that the update order for entities is consistent
    /// with the hierarchy structure. Parents should always run before their children,
    /// and siblings can run in parallel
    pub(super) fn run_stage_recursive_no_alloc(&mut self, world: &World, stage_id: StageID, recursion_stack:&mut Vec<EntityPtr>)
    {
        // TODO change this function to use rayon for parallel execution.
        // As long as the parent updates before its children, you can run it in parallel
        debug_assert!(self.is_spatial_entity(), "Can't recursively run stages for a non-spatial entity");
        debug_assert!(self.is_root(), "Entity to run recursively should be the root entity!");
        debug_assert!(recursion_stack.is_empty(), "Recursion stack should be empty or results will be inconsistent");

        // Run stage for the current node and push its children
        if self.is_stage_enabled(stage_id)
        {
            self.run_stage(world, stage_id);
        }

        for child in self.get_transform().as_ref().unwrap().children.iter()
        {
            recursion_stack.push(child.clone());
        }

        // Perform a dfs traversal over the children of this node.
        // We use iterative DFS to prevent recursion overhead
        while let Some(entity_lock) = recursion_stack.pop()
        {
            let mut entity = entity_lock.write();

            if entity.is_stage_enabled(stage_id)
            {
                entity.run_stage(world, stage_id);
            }

            for child in entity.get_transform().as_ref().unwrap().children.iter()
            {
                recursion_stack.push(child.clone());
            }
        }
    }

    /// Checks if this entity is a spatial entity
    #[inline(always)]
    pub fn is_spatial_entity(&self) -> bool
    {
        self.get_transform().is_some()
    }

    /// Checks if this entity is a root entity. 
    /// 
    /// Will panic if not a spatial entity.
    #[inline(always)]
    pub fn is_root(&self) -> bool
    {
        let transform = self.get_transform();
        debug_assert!(transform.is_some(), "Non-spatial entities can't have a root");

        transform.as_ref().unwrap().is_root()
    }

    /// Sets `parent_ptr` as the parent of `entity_ptr`
    /// 
    /// Used internally by the engine to re-parent an entity.
    /// 
    /// Users should enqueue its reparenting requests.
    /// # Panics
    /// If `parent` is not a spatial entity, or if this is not a spatial entity
    pub(super) fn set_parent(entity_ptr : EntityPtr, parent_ptr : EntityPtr)
    {
        debug_assert!(entity_ptr.read().is_spatial_entity(), "Can't set parent of non-spatial entity");
        debug_assert!(parent_ptr.read().is_spatial_entity(), "Parent entity should be a spatial entity as well");
        debug_assert!(entity_ptr.read().id != parent_ptr.read().id, "Entity can't be its own parent!");
        
        // Clear current parent. Note that you have to sub a few counters from the old parent before 
        // reparenting
        Entity::clear_parent(entity_ptr);

        let mut entity = entity_ptr.write();        
        let entity_transform = entity.get_transform_mut().unwrap();
        entity_transform.parent = Some(parent_ptr);

        // Make this node a child of the other node
        {
            let mut other = parent_ptr.write();
            let other_transform = other.get_transform_mut().unwrap();
            other_transform.children.push(entity_ptr);
        }

        // Now we have to go upwards updating the parent with the 
        // cached values of the amount of entities in hierarchy 
        // and entities that want to run some stage
        let mut next_parent_ptr = Some(parent_ptr);
        while next_parent_ptr.is_some()
        {
            let next_parent;
            {
                let mut parent = next_parent_ptr.as_mut().unwrap().write();
                let parent_transform = parent.get_transform_mut().unwrap();
                parent_transform.n_nodes += entity_transform.n_nodes;
                for i in 0..STAGE_COUNT
                {
                    // Add to the nodes per stage
                    parent_transform.stage_count[i]
                    .fetch_add(
                        entity_transform.stage_count[i].load(Ordering::Acquire), 
                        Ordering::Acquire
                    );
                }

                next_parent = parent_transform.parent;
            }
            
            next_parent_ptr = next_parent;
        }
    }

    /// Clears the parent of this entity, setting it to None.
    /// 
    /// Used internally by the engine to clear the parent of some entity. 
    /// 
    /// Users should enqueue its reparenting requests.
    /// # Panics
    /// if this is not a spatial entity
    pub(super) fn clear_parent(entity_ptr : EntityPtr)
    {
        debug_assert!(entity_ptr.read().is_spatial_entity(), "Can't clear parent of a non-spatial entity");
        let mut entity = entity_ptr.write();
        let transform = entity.get_transform_mut().unwrap();
        let mut parent = transform.parent;

        // Return if nothing to do
        if parent.is_none() 
        { return; }

        while parent.is_some()
        {
            let next_parent;
            {
                let mut parent = parent.as_mut().unwrap().write();
                let parent_transform = parent.get_transform_mut().unwrap();
                parent_transform.n_nodes -= transform.n_nodes;

                for i in 0..STAGE_COUNT
                {
                    parent_transform.stage_count[i]
                        .fetch_sub(
                            transform.stage_count[i].load(Ordering::Acquire), 
                            Ordering::Acquire
                        );
                }
                next_parent = parent_transform.parent;
            }

            parent = next_parent;
        }

        // Remove `entity_ptr` from the child list of `parent_ptr`
        {
            let mut parent = transform.parent.as_mut().unwrap().write();
            let parent_transform = parent
                                                .get_transform_mut()
                                                .expect("Parent should be a spatial entity");

            for i in 0..parent_transform.children.len()
            {
                let child = parent_transform.children[i];
                if child == entity_ptr
                {
                    parent_transform.children.swap_remove(i);
                    break;
                }
            }
        }
    }

    /// Initializes the transform datagroup for this entity.
    /// 
    /// # Panics
    /// If called in a non-spatial entity
    fn init_transform(&mut self)
    {
        let ls_stages = &self.ls_stage_enabled_map;
        let transform = self
            .get_transform()
            .expect("Can't init transform if entity has no transform");

        // Set the right value fot all counters
        for i in 0..STAGE_COUNT
        {
            let stage_enabled = ls_stages[i];
            if stage_enabled
            {
                transform.stage_count[i].store(1, Ordering::Release)
            }
        }
    }
}

impl std::fmt::Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dg_registry = DataGroupRegistry::get_global_registry().read();
        let ls_registry = LocalSystemRegistry::get_global_registry().read();

        #[derive(Debug)]
        #[allow(dead_code)] // To avoid warning due to Debug not counting as using fields
        struct LocalSystemRef {
            pub id: SystemClassID,
            pub name: &'static str,
        }

        #[derive(Debug)]
        #[allow(dead_code)] // To avoid warning due to Debug not counting as using fields
        struct LocalSystem {
            pub id: SystemClassID,
            pub name: &'static str,
            pub args: Vec<String>,
        }

        let mut local_system_map: IntMap<SystemClassID, LocalSystem> = IntMap::default();
        let mut local_system_ref_map: IntMap<SystemClassID, LocalSystemRef> = IntMap::default();
        for sys_id in &self.local_systems_map {
            let sys_entry = ls_registry.get_entry_by_id(*sys_id);

            let mut dependencies: Vec<String> = Vec::new();

            for dep in &sys_entry.dependencies {
                match dep {
                    Dependency::DataGroup(dg_id) => {
                        let dg_entry = dg_registry.get_entry_by_id(*dg_id);

                        let dg = self.get_datagroup_by_id(*dg_id);
                        match dg {
                            Some(_) => dependencies.push(dg_entry.name.to_owned()),
                            None => dependencies.push(format!("Error: {}", dg_entry.name)),
                        }
                    }
                    Dependency::OptionalDG(dg_id) => {
                        let dg_entry = dg_registry.get_entry_by_id(*dg_id);

                        let dg = self.get_datagroup_by_id(*dg_id);
                        match dg {
                            Some(_) => dependencies.push(dg_entry.name.to_owned()),
                            None => dependencies.push("None".to_owned()),
                        }
                    }
                }
            }

            local_system_map.insert(
                *sys_id,
                LocalSystem {
                    id: *sys_id,
                    name: sys_entry.name,
                    args: dependencies,
                },
            );

            local_system_ref_map.insert(
                *sys_id,
                LocalSystemRef {
                    id: *sys_id,
                    name: sys_entry.name,
                },
            );
        }

        #[derive(Debug)]
        #[allow(dead_code)] // To avoid warning due to Debug not counting as using fields
        struct Stage<'a> {
            pub local_systems: Vec<&'a LocalSystemRef>,
        }

        let mut stage_map: IntMap<StageID, Stage> = IntMap::default();

        let mut ls_stage_enabled_map: Vec<StageID> = Vec::new();
        ls_stage_enabled_map.reserve_exact(self.ls_stage_enabled_map.count_ones());

        self.ls_stage_enabled_map
            .iter()
            .enumerate()
            .for_each(|(stage, enabled)| {
                if *enabled {
                    ls_stage_enabled_map.push(stage as StageID);
                    stage_map.insert(
                        stage as StageID,
                        Stage {
                            local_systems: Vec::new(),
                        },
                    );
                }
            });

        let mut sorted_local_systems: Vec<SystemClassID> =
            self.local_systems_map.iter().copied().collect();
        sorted_local_systems.sort();

        for ls_id in &sorted_local_systems {
            let entry = ls_registry.get_entry_by_id(*ls_id);

            entry
                .functions
                .iter()
                .enumerate()
                .for_each(|(stage_id, fun)| {
                    let stage_id = stage_id as StageID;
                    match fun {
                        None => (),
                        Some(_) => {
                            let stage = stage_map.get_mut(&stage_id).unwrap();
                            stage
                                .local_systems
                                .push(local_system_ref_map.get(ls_id).unwrap())
                        }
                    }
                });
        }

        f.debug_struct("Entity")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("debug_info", &self.debug_info)
            .field("datagroups", &self.datagroups)
            .field("local_systems", &local_system_map.values())
            .field("ls_stage_enabled_map", &ls_stage_enabled_map)
            .field("stages", &stage_map)
            .finish()
    }
}



