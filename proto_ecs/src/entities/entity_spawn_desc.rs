use crate::core::ids;
use crate::data_group::{DataGroupID, DataGroupInitType, DataGroupRegistry};
use crate::entities::entity::MAX_DATAGROUP_LEN;
use crate::get_id;
use crate::systems::common::Dependency;
use crate::systems::global_systems::{GlobalSystemID, GlobalSystemRegistry};
use crate::systems::local_systems::{LocalSystemRegistry, SystemClassID};
use nohash_hasher::{IntMap, IntSet};

/// Description of an entity to be spawned
#[derive(Debug, Default)]
pub struct EntitySpawnDescription {
    pub(super) name: String,
    pub(super) debug_info: String,
    pub(super) data_groups: IntMap<DataGroupID, DataGroupInitType>,
    pub(super) local_systems: IntSet<SystemClassID>,
    pub(super) global_systems: IntSet<GlobalSystemID>,
}

impl EntitySpawnDescription {

    #[inline(always)]
    pub fn new() -> Self
    {
        Self::default()
    }

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
    /// Add a local system to an entity to be spawned
    pub fn add_global_system_by_id(&mut self, id: GlobalSystemID) -> bool {
        self.global_systems.insert(id)
    }

    #[inline(always)]
    /// Add a local system to an entity to be spawned
    pub fn add_global_system<S>(&mut self) -> bool
    where
        S: ids::IDLocator,
    {
        self.global_systems.insert(get_id!(S))
    }

    #[inline(always)]
    /// Get current local systems to be created for this entity
    pub fn get_global_systems(&self) -> &IntSet<GlobalSystemID> {
        &self.global_systems
    }

    #[inline(always)]
    /// Get if a local system will be used by this entity
    pub fn get_global_system_by_id(&self, id: GlobalSystemID) -> bool {
        self.get_global_systems().contains(&id)
    }

    #[inline(always)]
    /// Get if a local system will be used by this entity
    pub fn get_global_system<S>(&self) -> bool
    where
        S: ids::IDLocator,
    {
        self.get_global_system_by_id(get_id!(S))
    }

    /// Checks if the datagroups of this entity make sense, else panic
    pub fn check_datagroups_panic(&self) {
        assert!(
            self.get_datagroups().len() <= MAX_DATAGROUP_LEN as usize,
            "More datagroups than what the indexing type can support: {} (limit {})",
            self.get_datagroups().len(),
            MAX_DATAGROUP_LEN
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

    // Checks if the datagroups required by the global systems requested
    // by this entity are present
    fn check_global_systems_panic(&self) {
        let global_system_registry = GlobalSystemRegistry::get_global_registry().read();
        for &global_system in &self.global_systems {
            let gs_entry = global_system_registry.get_entry_by_id(global_system);
            for &datagroup in &gs_entry.dependencies {
                let dg_id = match datagroup {
                    Dependency::DataGroup(dg_id) => dg_id,
                    Dependency::OptionalDG(_) => {
                        continue;
                    } // nothing to check if they're optional
                };

                if self.get_datagroups().contains_key(&dg_id) {
                    // Everything ok, this entity has the required datagroup
                    continue;
                }

                let dg_name = DataGroupRegistry::get_global_registry()
                    .read()
                    .get_entry_by_id(dg_id)
                    .name;
                let gs_name = gs_entry.name;
                panic!(
                    "Entity doesn't have the datagroup '{dg_name}' required by the global system '{gs_name}', which is requested by the entity"
                );
            }
        }
    }

    /// Check if the entity to be spawned makes sense, else panic
    pub fn check_panic(&self) {
        self.check_datagroups_panic();
        self.check_local_systems_panic();
        self.check_global_systems_panic();
    }
}

/// Helpers to handle common uses cases for entity spawn descriptions
pub mod helpers {
    use crate::{
        core::common::InitDesc,
        core::ids,
        data_group::{
            DataGroup, DataGroupInitDescTrait, DataGroupInitType, DataGroupRegistryEntry,
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
            InitDesc::NoInit => DataGroupInitType::NoInit,
            InitDesc::NoArg => DataGroupInitType::NoArg,
            InitDesc::Arg => DataGroupInitType::Uninitialized(msg),
            InitDesc::OptionalArg => DataGroupInitType::OptionalArg(None),
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
            InitDesc::NoInit => assert!(
                matches!(init_param, DataGroupInitType::NoInit),
                "Datagroup '{}' expects a NoInit param, but found: {init_param:?}",
                entry.name
            ),
            InitDesc::NoArg => assert!(
                matches!(init_param, DataGroupInitType::NoArg),
                "Datagroup '{}' expects a NoArg param, but found: {init_param:?}",
                entry.name
            ),
            InitDesc::Arg => assert!(
                matches!(init_param, DataGroupInitType::Arg(_)),
                "Datagroup '{}' expects a Arg param, but found: {init_param:?}",
                entry.name
            ),
            InitDesc::OptionalArg => assert!(
                matches!(init_param, DataGroupInitType::OptionalArg(_)),
                "Datagroup '{}' expects a OptionalArg param, but found: {init_param:?}",
                entry.name
            ),
        }
    }
}
