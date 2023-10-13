use crate::data_group::{DataGroupID, DataGroupInitType, DataGroupMetadataLocator};
use crate::get_id;
use crate::local_systems::SystemClassID;
use nohash_hasher::{IntMap, IntSet};

/// Description of an entity to be spawned
#[derive(Debug, Default)]
pub struct EntitySpawnDescription {
    name: String,
    debug_info: String,
    data_groups: IntMap<DataGroupID, DataGroupInitType>,
    local_systems: IntSet<SystemClassID>,
}

impl EntitySpawnDescription {
    /// Set the name for this entity
    pub fn set_name(&mut self, new_name: String) {
        self.name = new_name;
    }

    /// Get the name for this entity
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Set the debug info (e. g. which system created it) for this entity
    pub fn set_debug_info(&mut self, new_debug_info: String) {
        self.debug_info = new_debug_info;
    }

    /// Get the debug info (e. g. which system created it) for this entity
    pub fn get_debug_info(&self) -> &str {
        &self.debug_info
    }

    /// Add a datagroup to an entity to be spawned
    /// Normally this should only be called by the internal engine. Prefer to use DataGroup::Prepare
    pub fn add_datagroup_by_id(
        &mut self,
        id: DataGroupID,
        init_args: DataGroupInitType,
    ) -> Option<DataGroupInitType> {
        self.data_groups.insert(id, init_args)
    }

    /// Add a datagroup to an entity to be spawned
    /// Normally this should only be called by the internal engine. Prefer to use DataGroup::Prepare
    pub fn add_datagroup<D>(&mut self, init_args: DataGroupInitType) -> Option<DataGroupInitType>
    where
        D: DataGroupMetadataLocator,
    {
        self.add_datagroup_by_id(get_id!(D), init_args)
    }

    /// Get current datagroups to be created for this entity
    pub fn get_datagroups(&self) -> &IntMap<DataGroupID, DataGroupInitType> {
        &self.data_groups
    }

    /// Add a local system to an entity to be spawned
    pub fn add_local_system_by_id(&mut self, id: SystemClassID) -> bool {
        self.local_systems.insert(id)
    }

    /// Get current local systems to be created for this entity
    pub fn get_local_systems(&self) -> &IntSet<SystemClassID> {
        &self.local_systems
    }
}
