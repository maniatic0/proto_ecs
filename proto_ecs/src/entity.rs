use crate::{
    data_group::{DataGroup, DataGroupID},
    entity_spawn_desc::EntitySpawnDescription,
};
use proto_ecs::local_systems::{StageID, SystemClassID, SystemFn, STAGE_COUNT};

use bitvec::prelude::BitArr;
use nohash_hasher::IntSet;
use vector_map::{set::VecSet, VecMap};

pub type EntityID = u64;

/// The invalid entity ID
pub const INVALID_ENTITY_ID: EntityID = EntityID::MAX;

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

    datagroups: DataGroupMap,

    local_systems_map: LocalSystemMap,
    stage_enabled_map: StageEnabledMap,
    stage_map: StageMap,

    parent: EntityID,
    children: ChildrenMap,
}

impl Entity {
    pub fn new(id: EntityID, spawn_desc: EntitySpawnDescription) -> Self {
        todo!()
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
    pub fn get_children(&self) -> &ChildrenMap {
        &self.children
    }

    pub fn run_stage(&mut self, stage_id: StageID) {
        debug_assert!(
            self.is_stage_enabled(stage_id),
            "Check if the stage is enabled before running it!"
        );
    }
}
