use crate::data_group::DataGroup;
use crate::entities::entity::{DataGroupIndexingType, EntityID};
use crate::entities::entity_system::World;
pub use ecs_macros::register_local_system;
/// Local systems are basically functions that operate on datagroups from
/// an entity. To define a local system, the user should be able to
/// write a function with datagroups it expects as parameters and
/// annotate a macro attribute that will register that system. E.g:
///
/// #[local_system]
/// pub fn MySystem(animation : &mut AnimationDatagroup, mesh : &mut MeshDatagroup)
/// { ... }
use lazy_static::lazy_static;
use proto_ecs::core::casting::CanCast;
use proto_ecs::core::{ids, locking::RwLock};
use proto_ecs::get_id;
use topological_sort::TopologicalSort;

use proto_ecs::systems::common::*;

pub type SystemClassID = u32;

pub const INVALID_SYSTEM_CLASS_ID: SystemClassID = SystemClassID::MAX;

pub trait CanRun<Args> {
    fn run(&mut self, args: Args);
}

pub trait LocalSystem: LocalSystemMeta + CanCast {
    fn run(datagroups: Box<dyn DataGroup>);
}

pub trait LocalSystemMeta {
    fn get_id(&self) -> SystemClassID;
}

pub type LocalSystemFactory = fn() -> Box<dyn LocalSystem>;

pub type SystemFn = fn(&World, EntityID, &[DataGroupIndexingType], &mut [Box<dyn DataGroup>]) -> ();

/// Stage Map type
pub type LSStageMap = StageMap<SystemFn>;

/// Empty stage map
pub const EMPTY_STAGE_MAP: LSStageMap = [None; STAGE_COUNT];

pub trait LocalSystemDesc {
    const NAME: &'static str;
    const NAME_CRC: u32;
}

#[derive(Debug)]
pub struct LocalSystemRegistryEntry {
    pub id: SystemClassID,
    pub name: &'static str,
    pub name_crc: u32,
    pub dependencies: Vec<Dependency>,
    pub functions: LSStageMap,
    pub before: Vec<SystemClassID>,
    pub after: Vec<SystemClassID>,
    pub set_id_fn: fn(SystemClassID), // Only used for init, don't use it manually
}

#[derive(Debug, Default)]
pub struct LocalSystemRegistry {
    entries: Vec<LocalSystemRegistryEntry>,
    is_initialized: bool,
}

impl LocalSystemRegistry {
    #[inline]
    pub fn new() -> Self {
        LocalSystemRegistry::default()
    }

    #[inline]
    fn get_temp_global_registry() -> &'static RwLock<TempRegistryLambdas> {
        &LOCAL_SYSTEM_REGISTRY_TEMP
    }

    pub fn register_lambda(lambda: TempRegistryLambda) {
        LocalSystemRegistry::get_temp_global_registry()
            .write()
            .push(lambda)
    }

    #[inline]
    pub fn get_global_registry() -> &'static RwLock<Self> {
        &GLOBAL_SYSTEM
    }

    pub fn register(&mut self, entry: LocalSystemRegistryEntry) {
        self.entries.push(entry);
    }

    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Initialize the global registry
    pub fn initialize() {
        let mut registry = LocalSystemRegistry::get_global_registry().write();
        assert!(
            !registry.is_initialized,
            "Local System registry was already initialized!"
        );

        let mut locals_register_fns = TempRegistryLambdas::new();
        let mut globals_register_fns = LocalSystemRegistry::get_temp_global_registry().write();

        // Clear globals
        std::mem::swap(&mut locals_register_fns, &mut globals_register_fns);

        registry.init(locals_register_fns);
    }

    /// Initialize this registry entry
    pub fn init(&mut self, registry_fns: TempRegistryLambdas) {
        registry_fns.into_iter().for_each(|lambda| lambda(self));
        self.set_toposort_ids();

        self.entries
            .sort_unstable_by(|this, other| this.id.cmp(&other.id));

        self.is_initialized = true;
    }

    #[inline]
    pub fn get_entry_by_id(&self, id: SystemClassID) -> &LocalSystemRegistryEntry {
        debug_assert!((id as usize) < self.entries.len(), "Invalid ID");
        &self.entries[id as usize]
    }

    /// Set ids for local systems based on the topological ordering
    /// generated by the `before` and `after` dependencies. Local systems
    /// can then be sorted by id to get the order in which they should be run
    fn set_toposort_ids(&mut self) {
        if self.entries.is_empty() {
            return; // Nothing to do if there are no entries
        }

        let mut ts: TopologicalSort<SystemClassID> = TopologicalSort::new();
        let source_node = SystemClassID::default();

        for entry in self.entries.iter() {
            let entry_crc = entry.name_crc;
            ts.add_dependency(source_node, entry_crc);

            // Sanity check
            debug_assert!(
                source_node != entry.name_crc,
                "Source node should be a value never reachable by the crc"
            );
            for &other_crc in entry.before.iter() {
                ts.add_dependency(entry_crc, other_crc);
            }

            for &other_crc in entry.after.iter() {
                ts.add_dependency(other_crc, entry_crc);
            }
        }

        let source_node_vec = ts.pop_all();
        debug_assert!(
            source_node_vec.len() == 1,
            "The first dependency should be only the source node"
        );
        debug_assert!(
            source_node_vec[0] == source_node,
            "The first dependency should be the source node"
        );
        let mut dependency_order = vec![];
        while !ts.is_empty() {
            let mut non_dependents = ts.pop_all();
            if non_dependents.is_empty() && !ts.is_empty() {
                // If there's cyclic dependencies,
                // then the popped list is empty and ts.len > 0,
                // See: https://docs.rs/topological-sort/latest/topological_sort/struct.TopologicalSort.html#method.pop_all
                // TODO: better error handling, report cyclic dependencies
                panic!("Cyclic dependencies between local systems!");
            }

            // Non-dependents are elements that do not depend on anything else.
            // Sort them by value to get a deterministic ordering each time.
            // Since they don't depend on each other, the actual order doesn't matter.
            non_dependents.sort();
            dependency_order.extend(non_dependents);
        }

        for entry in self.entries.iter_mut() {
            let id = dependency_order
                .iter()
                .position(|&crc| entry.name_crc == crc)
                .unwrap();
            entry.id = id as SystemClassID;
            (entry.set_id_fn)(id as SystemClassID);
        }
    }

    /// Get the entry for a specific LocalSystem
    pub fn get_entry<S>(&self) -> &LocalSystemRegistryEntry
    where
        S: ids::IDLocator + LocalSystemDesc,
    {
        self.get_entry_by_id(get_id!(S))
    }

    pub fn set_dependencies<S>(&mut self, before: Vec<SystemClassID>, after: Vec<SystemClassID>)
    where
        S: ids::IDLocator + LocalSystemDesc,
    {
        // We won't allow changing dependencies in runtime
        debug_assert!(
            !self.is_initialized,
            "You can only set dependencies before initializing local systems"
        );
        let entry = &mut self.entries[get_id!(S) as usize];
        entry.before = before;
        entry.after = after;
    }
}

pub type TempRegistryLambda = Box<dyn FnOnce(&mut LocalSystemRegistry) + Sync + Send + 'static>;
type TempRegistryLambdas = Vec<TempRegistryLambda>;

lazy_static! {

    // This registry holds functions that register a local system.
    // It's filled before main so that we choose when to call this functions.
    static ref LOCAL_SYSTEM_REGISTRY_TEMP: RwLock<TempRegistryLambdas> =
        RwLock::from(TempRegistryLambdas::new());
}

lazy_static! {
    static ref GLOBAL_SYSTEM: RwLock<LocalSystemRegistry> =
        RwLock::from(LocalSystemRegistry::new());
}
