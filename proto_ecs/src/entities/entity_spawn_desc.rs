use crate::core::ids;
use crate::data_group::{DataGroupID, DataGroupInitType, DataGroupRegistry};
use crate::entities::entity::{ChildrenMap, EntityID, INVALID_ENTITY_ID, MAX_DATAGROUP_INDEX};
use crate::get_id;
use crate::local_systems::{Dependency, LocalSystemRegistry, SystemClassID};
use nohash_hasher::{IntMap, IntSet};

/// Description of an entity to be spawned
#[derive(Debug)]
pub struct EntitySpawnDescription {
    pub(super) name: String,
    pub(super) debug_info: String,
    pub(super) data_groups: IntMap<DataGroupID, DataGroupInitType>,
    pub(super) local_systems: IntSet<SystemClassID>,
    pub(super) parent: EntityID,
    pub(super) children: ChildrenMap,
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
    pub fn get_datagroup_by_id(&self, id: DataGroupID) -> Option<&DataGroupInitType> {
        self.get_datagroups().get(&id)
    }

    #[inline(always)]
    /// Get current data group init data
    pub fn get_datagroup<D>(&self) -> Option<&DataGroupInitType>
    where
        D: ids::IDLocator,
    {
        self.get_datagroup_by_id(get_id!(D))
    }

    #[inline(always)]
    /// Get current datagroups to be created for this entity
    /// Internal method to avoid modifications to the presence of a datagroup
    fn get_datagroups_mut(&mut self) -> &mut IntMap<DataGroupID, DataGroupInitType> {
        &mut self.data_groups
    }

    #[inline]
    /// Get current data group init data
    pub fn get_datagroup_mut_by_id(&mut self, id: DataGroupID) -> Option<&mut DataGroupInitType> {
        self.get_datagroups_mut().get_mut(&id)
    }

    #[inline(always)]
    /// Get current data group init data
    pub fn get_datagroup_mut<D>(&mut self) -> Option<&mut DataGroupInitType>
    where
        D: ids::IDLocator,
    {
        self.get_datagroup_mut_by_id(get_id!(D))
    }

    #[inline(always)]
    /// Add a local system to an entity to be spawned
    pub fn add_local_system_by_id(&mut self, id: SystemClassID) -> bool {
        self.local_systems.insert(id)
    }

    #[inline(always)]
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
    pub fn get_local_system_by_id(&self, id: SystemClassID) -> bool {
        self.get_local_systems().contains(&id)
    }

    #[inline(always)]
    /// Get if a local system will be used by this entity
    pub fn get_local_system<S>(&self) -> bool
    where
        S: ids::IDLocator,
    {
        self.get_local_system_by_id(get_id!(S))
    }

    #[inline(always)]
    /// Get the parent of this entity
    pub fn get_parent(&self) -> EntityID {
        self.parent
    }

    #[inline(always)]
    /// Set the parent of this entity
    pub fn set_parent(&mut self, parent: EntityID) {
        self.parent = parent
    }

    #[inline(always)]
    /// Get the children of this entity
    pub fn get_children(&self) -> &ChildrenMap {
        &self.children
    }

    #[inline(always)]
    /// Get the children of this entity
    pub fn get_children_mut(&mut self) -> &mut ChildrenMap {
        &mut self.children
    }

    #[inline(always)]
    /// Add a child to this entity
    pub fn add_child(&mut self, child: EntityID) {
        self.children.insert(child);
    }

    #[inline(always)]
    /// Remove a child to this entity
    pub fn remove_child(&mut self, child: EntityID) {
        self.children.remove(&child);
    }

    /// Checks if the datagroups of this entity make sense, else panic
    pub fn check_datagroups_panic(&self) {
        assert!(
            self.get_datagroups().len() <= MAX_DATAGROUP_INDEX as usize,
            "More datagroups than what the indexing type can support: {} (limit {})",
            self.get_datagroups().len(),
            MAX_DATAGROUP_INDEX
        );

        let registry = DataGroupRegistry::get_global_registry().read();

        self.get_datagroups().iter().for_each(|(id, init_param)| {
            let entry = registry.get_entry_by_id(*id);

            helpers::check_init_params_panic(init_param, entry)
        });
    }

    /// Checks if the local systems of this entity have their dependencies met
    pub fn check_local_systems_panic(&self) {
        let registry = LocalSystemRegistry::get_global_registry().read();

        self.get_local_systems().iter().for_each(|id| {
            let entry = registry.get_entry_by_id(*id);

            entry.dependencies.iter().for_each(|dep| {
                let dg_id = match dep {
                    Dependency::DataGroup(id) => id,
                    Dependency::OptionalDG(_) => return,
                };

                if self.get_datagroups().contains_key(dg_id) {
                    return;
                }

                let dg_registry = DataGroupRegistry::get_global_registry().read();

                panic!(
                    "Local System '{}' is missing dependency Datagroup '{}'",
                    entry.name,
                    dg_registry.get_entry_by_id(*dg_id).name
                );
            });
        });
    }

    /// Check if the entity to be spawned makes sense, else panic
    pub fn check_panic(&self) {
        self.check_datagroups_panic();
        self.check_local_systems_panic();
    }
}

impl Default for EntitySpawnDescription {
    fn default() -> Self {
        Self {
            name: Default::default(),
            debug_info: Default::default(),
            data_groups: Default::default(),
            local_systems: Default::default(),
            parent: INVALID_ENTITY_ID,
            children: Default::default(),
        }
    }
}

/// Helpers to handle common uses cases for entity spawn descriptions
pub mod helpers {
    use crate::{
        core::ids,
        data_group::{
            DataGroup, DataGroupInitDesc, DataGroupInitDescTrait, DataGroupInitType,
            DataGroupRegistryEntry,
        },
        get_id,
    };

    use super::EntitySpawnDescription;

    /// Add an uninitialized datagroup dependency to the spawn description
    pub fn local_system_try_add_datagroup<D>(
        spawn_desc: &mut EntitySpawnDescription,
        msg: &'static str,
    ) where
        D: ids::IDLocator + DataGroup + DataGroupInitDescTrait,
    {
        let default_init = match <D as DataGroupInitDescTrait>::INIT_DESC {
            DataGroupInitDesc::NoInit => DataGroupInitType::NoInit,
            DataGroupInitDesc::NoArg => DataGroupInitType::NoArg,
            DataGroupInitDesc::Arg => DataGroupInitType::Uninitialized(msg),
            DataGroupInitDesc::OptionalArg => DataGroupInitType::OptionalArg(None),
        };

        spawn_desc
            .get_datagroups_mut()
            .entry(get_id!(D))
            .or_insert_with(|| default_init);
    }

    /// Checks if the init params of a DataGroup matches what it expects them to be. If they are not correct, it panics
    pub fn check_init_params_panic(init_param: &DataGroupInitType, entry: &DataGroupRegistryEntry) {
        if let DataGroupInitType::Uninitialized(msg) = init_param {
            panic!(
                "Found Uninitialized init param for DataGroup '{}' params: {msg}",
                entry.name
            );
        }

        match entry.init_desc {
            DataGroupInitDesc::NoInit => assert!(
                matches!(init_param, DataGroupInitType::NoInit),
                "Datagroup '{}' expects a NoInit param, but found: {init_param:?}",
                entry.name
            ),
            DataGroupInitDesc::NoArg => assert!(
                matches!(init_param, DataGroupInitType::NoArg),
                "Datagroup '{}' expects a NoArg param, but found: {init_param:?}",
                entry.name
            ),
            DataGroupInitDesc::Arg => assert!(
                matches!(init_param, DataGroupInitType::Arg(_)),
                "Datagroup '{}' expects a Arg param, but found: {init_param:?}",
                entry.name
            ),
            DataGroupInitDesc::OptionalArg => assert!(
                matches!(init_param, DataGroupInitType::OptionalArg(_)),
                "Datagroup '{}' expects a OptionalArg param, but found: {init_param:?}",
                entry.name
            ),
        }
    }
}