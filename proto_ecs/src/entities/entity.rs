use crate::{
    data_group::{DataGroup, DataGroupID},
    entities::entity_spawn_desc::EntitySpawnDescription,
};
use proto_ecs::local_systems::{StageID, SystemClassID, SystemFn, STAGE_COUNT};

use bitvec::prelude::BitArr;
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
    pub fn new(id: EntityID, spawn_desc: EntitySpawnDescription) -> Self {
        let EntitySpawnDescription {
            name,
            debug_info,
            data_groups,
            local_systems,
            parent,
            children,
        } = spawn_desc;

        Self {
            id,
            name,
            debug_info,
            datagroups: todo!(),
            local_systems_map: todo!(),
            stage_enabled_map: todo!(),
            stage_map: todo!(),
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
    }
}
