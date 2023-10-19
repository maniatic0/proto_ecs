use crate::{
    core::ids::IDLocator,
    data_group::{DataGroup, DataGroupID, DataGroupInitType, DataGroupRegistry},
    entities::entity_spawn_desc::EntitySpawnDescription,
    get_id,
    local_systems::{Dependency, LocalSystemRegistry, LocalSystemDesc},
};
use proto_ecs::local_systems::{StageID, SystemClassID, SystemFn, STAGE_COUNT};

use bitvec::prelude::{BitArr, BitArray};
use nohash_hasher::{IntMap, IntSet};
use vector_map::{set::VecSet, VecMap};

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
pub const MAX_DATAGROUP_INDEX: DataGroupIndexingType = INVALID_DATAGROUP_INDEX - 1;

/// Map type used by entities to store what local systems it has
pub type LocalSystemMap = IntSet<SystemClassID>;

/// Map type used by entities to store local systems' enabled stages
pub type StageEnabledMap = BitArr!(for STAGE_COUNT);

/// Map type used by entities to store local systems' execution functions per stage
pub type StageMap = VecMap<StageID, Vec<(Vec<DataGroupIndexingType>, SystemFn)>>;

/// Map type used by entities to store the reference to its children
pub type ChildrenMap = VecSet<EntityID>;

pub struct Entity {
    id: EntityID,
    name: String,
    debug_info: String,

    datagroups: DataGroupVec,

    local_systems_map: LocalSystemMap,
    stage_enabled_map: StageEnabledMap,
    stage_map: StageMap,

    parent: EntityID,
    children: ChildrenMap,
}

impl Entity {
    pub fn init(id: EntityID, spawn_desc: EntitySpawnDescription) -> Self {
        let EntitySpawnDescription {
            name,
            debug_info,
            data_groups,
            local_systems,
            parent,
            children,
        } = spawn_desc;

        // Init Datagroups
        let dg_registry = DataGroupRegistry::get_global_registry().read();
        let mut datagroups = DataGroupVec::new();

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
        }

        // Sort them to be able to use binary search
        datagroups.sort_by_key(|dg| dg.get_id());

        // Build temp map for their positions (for Local Systems lookup)
        let mut dg_to_pos_map: IntMap<DataGroupID, DataGroupIndexingType> = IntMap::default();
        for (pos, dg_id) in datagroups.iter().enumerate() {
            dg_to_pos_map.insert(dg_id.get_id(), pos as DataGroupIndexingType);
        }

        // Build stage information and collect datagroup indices
        let mut stage_enabled_map = BitArray::ZERO;
        let mut stage_map = StageMap::new();

        let mut sorted_local_systems: Vec<SystemClassID> = local_systems.iter().copied().collect();
        sorted_local_systems.sort();
        let ls_registry = LocalSystemRegistry::get_global_registry().read();
        for id in &sorted_local_systems {
            let entry = ls_registry.get_entry_by_id(*id);

            entry
                .functions
                .iter()
                .enumerate()
                .for_each(|(stage_id, fun)| {
                    let stage_id = stage_id as StageID;
                    match fun {
                        None => (),
                        Some(fun) => {
                            if !stage_enabled_map[stage_id as usize] {
                                stage_enabled_map.set(stage_id as usize, true);
                                stage_map.insert(stage_id, Vec::new());
                            }

                            let mut dependency_ids: Vec<DataGroupIndexingType> = Vec::new();
                            dependency_ids.reserve_exact(entry.dependencies.len());

                            for dep in &entry.dependencies {
                                match dep {
                                    Dependency::DataGroup(dg_id) => {
                                        dependency_ids.push(*dg_to_pos_map.get(dg_id).expect(
                                            "Local System is missing datagroup dependency!",
                                        ))
                                    }
                                    Dependency::OptionalDG(dg_id) => match dg_to_pos_map.get(dg_id)
                                    {
                                        Some(pos) => dependency_ids.push(*pos),
                                        None => dependency_ids.push(INVALID_DATAGROUP_INDEX),
                                    },
                                }
                            }

                            debug_assert_eq!(
                                dependency_ids.capacity(),
                                entry.dependencies.len(),
                                "Unexpected extra slack!"
                            );

                            let stage = stage_map.get_mut(&stage_id).unwrap();
                            stage.push((dependency_ids, *fun));
                        }
                    }
                });
        }

        Self {
            id,
            name,
            debug_info,
            datagroups,
            local_systems_map: local_systems,
            stage_enabled_map,
            stage_map,
            parent,
            children,
        }
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
    pub fn get_datagroup_by_id(&self, id: DataGroupID) -> Option<&Box<dyn DataGroup>> {
        let pos = self.datagroups.binary_search_by_key(&id, |dg| dg.get_id());
        match pos {
            Ok(pos) => Some(&self.datagroups[pos]),
            Err(_) => None,
        }
    }

    #[inline(always)]
    pub fn get_datagroup<DG>(&self) -> Option<&Box<dyn DataGroup>>
    where
        DG: IDLocator + DataGroup,
    {
        self.get_datagroup_by_id(get_id!(DG))
    }

    #[inline(always)]
    pub fn get_local_systems(&self) -> &LocalSystemMap {
        &self.local_systems_map
    }

    #[inline(always)]
    pub fn contains_local_system_by_id(&self, id : SystemClassID) -> bool
    {
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
    pub fn get_stage_enabled_map(&self) -> &StageEnabledMap {
        &self.stage_enabled_map
    }

    #[inline(always)]
    /// If a stage is enabled for this entity
    pub fn is_stage_enabled(&self, stage_id: StageID) -> bool {
        self.stage_enabled_map[stage_id as usize]
    }

    #[inline(always)]
    pub fn get_parent(&self) -> EntityID {
        self.parent
    }

    #[inline(always)]
    /// Only to be used by the entity system
    pub(super) fn set_parent(&mut self, parent: EntityID) {
        self.parent = parent
    }

    #[inline(always)]
    pub fn get_children(&self) -> &ChildrenMap {
        &self.children
    }

    #[inline(always)]
    /// Only to be used by the entity system
    pub(super) fn get_children_mut(&mut self) -> &mut ChildrenMap {
        &mut self.children
    }

    #[inline(always)]
    /// Add a child to this entity
    /// Only to be used by the entity system
    pub(super) fn add_child(&mut self, child: EntityID) {
        self.children.insert(child);
    }

    #[inline(always)]
    /// Remove a child to this entity
    /// Only to be used by the entity system
    pub(super) fn remove_child(&mut self, child: EntityID) {
        self.children.remove(&child);
    }

    /// Runs a stage. Note that it panics if the stage is not enabled
    /// Only to be called by the entity system
    pub(super) fn run_stage(&mut self, stage_id: StageID) {
        debug_assert!(
            self.is_stage_enabled(stage_id),
            "Check if the stage is enabled before running it!"
        );

        let stage = self
            .stage_map
            .get_mut(&stage_id)
            .expect("Unitialized Entity or Entity in undefined state!");

        for (dependencies, local_sys_fun) in stage {
            (local_sys_fun)(&dependencies, &mut self.datagroups)
        }
    }
}
