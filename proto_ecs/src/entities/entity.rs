use crate::{
    core::{
        casting::{cast, CanCast},
        ids::IDLocator,
    },
    data_group::{DataGroup, DataGroupID, DataGroupInitType, DataGroupRegistry},
    entities::entity_spawn_desc::EntitySpawnDescription,
    get_id,
    systems::local_systems::{LocalSystemDesc, LocalSystemRegistry},
    systems::common::Dependency
};
use proto_ecs::systems::local_systems::{SystemClassID, SystemFn};
use proto_ecs::systems::common::{StageID, STAGE_COUNT};

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

/// From where to get the local system datagroup indices
type LocalSystemIndexingVec = Vec<DataGroupIndexingType>;

/// Map type used by entities to store local systems' execution functions per stage
pub type StageMap = VecMap<StageID, Vec<(DataGroupIndexingType, SystemFn)>>;

/// Map type used by entities to store the reference to its children
pub type ChildrenMap = VecSet<EntityID>;

pub struct Entity {
    id: EntityID,
    name: String,
    debug_info: String,

    datagroups: DataGroupVec,

    local_systems_indices: LocalSystemIndexingVec,
    local_systems_map: LocalSystemMap,
    stage_enabled_map: StageEnabledMap,
    stage_map: StageMap,
}

impl Entity {
    pub fn init(id: EntityID, spawn_desc: EntitySpawnDescription) -> Self {
        let EntitySpawnDescription {
            name,
            debug_info,
            data_groups,
            local_systems,
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
                            if !stage_enabled_map[stage_id as usize] {
                                stage_enabled_map.set(stage_id as usize, true);
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

        Self {
            id,
            name,
            debug_info,
            datagroups,
            local_systems_indices,
            local_systems_map: local_systems,
            stage_enabled_map,
            stage_map,
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
    pub fn get_datagroup_by_id(&self, id: DataGroupID) -> Option<&dyn DataGroup> {
        let pos = self.datagroups.binary_search_by_key(&id, |dg| dg.get_id());
        match pos {
            Ok(pos) => Some(self.datagroups[pos].as_ref()),
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
    pub fn get_local_systems(&self) -> &LocalSystemMap {
        &self.local_systems_map
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
    pub fn get_stage_enabled_map(&self) -> &StageEnabledMap {
        &self.stage_enabled_map
    }

    #[inline(always)]
    /// If a stage is enabled for this entity
    pub fn is_stage_enabled(&self, stage_id: StageID) -> bool {
        self.stage_enabled_map[stage_id as usize]
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

        let mut indices_start: usize = 0;

        for (indices_num, local_sys_fun) in stage {
            let indices_num = *indices_num as usize;
            (local_sys_fun)(
                self.id,
                &self.local_systems_indices[indices_start..(indices_start + indices_num)],
                &mut self.datagroups,
            );
            indices_start += indices_num;
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

        let mut stage_enabled_map: Vec<StageID> = Vec::new();
        stage_enabled_map.reserve_exact(self.stage_enabled_map.count_ones());

        self.stage_enabled_map
            .iter()
            .enumerate()
            .for_each(|(stage, enabled)| {
                if *enabled {
                    stage_enabled_map.push(stage as StageID);
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
            .field("stage_enabled_map", &stage_enabled_map)
            .field("stages", &stage_map)
            .finish()
    }
}
