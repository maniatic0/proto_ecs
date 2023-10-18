use crate::{
    data_group::{DataGroup, DataGroupID, DataGroupInitType, DataGroupRegistry},
    entities::entity_spawn_desc::EntitySpawnDescription,
    local_systems::LocalSystemRegistry,
};
use proto_ecs::local_systems::{StageID, SystemClassID, SystemFn, STAGE_COUNT};

use bitvec::prelude::{BitArr, BitArray};
use nohash_hasher::IntSet;
use vector_map::{set::VecSet, VecMap};

pub type EntityID = u64;

/// The invalid entity ID
pub const INVALID_ENTITY_ID: EntityID = 0;

/// Map type used by entities to store datagroups
pub type DataGroupMap = VecMap<DataGroupID, Box<dyn DataGroup>>;

/// Map type used by entities to store what local systems it has
pub type LocalSystemMap = IntSet<SystemClassID>;

/// Map type used by entities to store local systems' enabled stages
pub type StageEnabledMap = BitArr!(for STAGE_COUNT);

/// Map type used by entities to store local systems' execution functions per stage
pub type StageMap = VecMap<StageID, Vec<SystemFn>>;

/// Map type used by entities to store the reference to its children
pub type ChildrenMap = VecSet<EntityID>;

pub struct Entity {
    id: EntityID,
    name: String,
    debug_info: String,

    datagroups: DataGroupMap,

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

        let dg_registry = DataGroupRegistry::get_global_registry().read();
        let mut datagroups = DataGroupMap::new();

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

            datagroups.insert(id, new_dg);
        }

        let mut stage_enabled_map = BitArray::ZERO;
        let mut stage_map = StageMap::new();

        let ls_registry = LocalSystemRegistry::get_global_registry().read();
        for id in &local_systems {
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

                            let stage = stage_map.get_mut(&stage_id).unwrap();
                            stage.push(*fun);
                        }
                    }
                });
        }

        // TODO: Add datagroup index math here and to the struct
        // TODO: Add local system sorting here

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
    pub fn get_datagroups(&self) -> &DataGroupMap {
        &self.datagroups
    }

    #[inline(always)]
    pub fn get_local_systems(&self) -> &LocalSystemMap {
        &self.local_systems_map
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

    pub fn run_stage(&mut self, stage_id: StageID) {
        debug_assert!(
            self.is_stage_enabled(stage_id),
            "Check if the stage is enabled before running it!"
        );

        let stage = self
            .stage_map
            .get_mut(&stage_id)
            .expect("Unitialized Entity or Entity in undefined state!");
    }
}
