use crate::core::ids;
use crate::data_group::{DataGroupID, DataGroupInitType};
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
    #[inline]
    /// Set the name for this entity
    pub fn set_name(&mut self, new_name: String) {
        self.name = new_name;
    }

    #[inline]
    /// Get the name for this entity
    pub fn get_name(&self) -> &str {
        &self.name
    }

    #[inline]
    /// Set the debug info (e. g. which system created it) for this entity
    pub fn set_debug_info(&mut self, new_debug_info: String) {
        self.debug_info = new_debug_info;
    }

    #[inline]
    /// Get the debug info (e. g. which system created it) for this entity
    pub fn get_debug_info(&self) -> &str {
        &self.debug_info
    }

    #[inline]
    /// Add a datagroup to an entity to be spawned
    /// Normally this should only be called by the internal engine. Prefer to use DataGroup::Prepare
    pub fn add_datagroup_by_id(
        &mut self,
        id: DataGroupID,
        init_args: DataGroupInitType,
    ) -> Option<DataGroupInitType> {
        self.data_groups.insert(id, init_args)
    }

    #[inline(always)]
    /// Add a datagroup to an entity to be spawned
    /// Normally this should only be called by the internal engine. Prefer to use DataGroup::Prepare
    pub fn add_datagroup<D>(&mut self, init_args: DataGroupInitType) -> Option<DataGroupInitType>
    where
        D: ids::IDLocator,
    {
        self.add_datagroup_by_id(get_id!(D), init_args)
    }

    #[inline(always)]
    /// Get current datagroups to be created for this entity
    pub fn get_datagroups(&self) -> &IntMap<DataGroupID, DataGroupInitType> {
        &self.data_groups
    }

    #[inline]
    /// Get current data group init data
    pub fn get_datagroup_by_id(&self, id: &DataGroupID) -> Option<&DataGroupInitType> {
        self.get_datagroups().get(id)
    }

    #[inline]
    /// Get current data group init data
    pub fn get_datagroup<D>(&self) -> Option<&DataGroupInitType>
    where
        D: ids::IDLocator,
    {
        self.get_datagroup_by_id(&get_id!(D))
    }

    #[inline(always)]
    /// Get current datagroups to be created for this entity
    pub fn get_datagroups_mut(&mut self) -> &mut IntMap<DataGroupID, DataGroupInitType> {
        &mut self.data_groups
    }

    #[inline]
    /// Get current data group init data
    pub fn get_datagroup_mut_by_id(&mut self, id: &DataGroupID) -> Option<&mut DataGroupInitType> {
        self.get_datagroups_mut().get_mut(id)
    }

    #[inline]
    /// Get current data group init data
    pub fn get_datagroup_mut<D>(&mut self) -> Option<&mut DataGroupInitType>
    where
        D: ids::IDLocator,
    {
        self.get_datagroup_mut_by_id(&get_id!(D))
    }

    #[inline(always)]
    /// Add a local system to an entity to be spawned
    pub fn add_local_system_by_id(&mut self, id: SystemClassID) -> bool {
        self.local_systems.insert(id)
    }

    #[inline]
    /// Add a local system to an entity to be spawned
    pub fn add_local_system<S>(&mut self) -> bool
    where
        S: ids::IDLocator,
    {
        self.local_systems.insert(get_id!(S))
    }

    #[inline(always)]
    /// Get current local systems to be created for this entity
    pub fn get_local_systems(&self) -> &IntSet<SystemClassID> {
        &self.local_systems
    }

    #[inline(always)]
    /// Get if a local system will be used by this entity
    pub fn get_local_system_by_id(&self, id: &SystemClassID) -> bool {
        self.get_local_systems().contains(id)
    }

    #[inline]
    /// Get if a local system will be used by this entity
    pub fn get_local_system<S>(&self) -> bool
    where
        S: ids::IDLocator,
    {
        self.get_local_system_by_id(&get_id!(S))
    }
}
