use lazy_static::lazy_static;
use parking_lot::RwLock;
use proto_ecs::core::ids;
use proto_ecs::get_id;
use proto_ecs::systems::common::*;
use topological_sort::TopologicalSort;
use std::collections::HashMap;
use proto_ecs::entities::entity;
use proto_ecs::core::casting::CanCast;
use proto_ecs::core::common::InitDesc;

pub use ecs_macros::register_global_system;


// TODO Change to a smaller type
pub type GlobalSystemID = u32; 

pub const INVALID_GLOBAL_SYSTEM_CLASS_ID: GlobalSystemID = GlobalSystemID::MAX;

// TODO Change for the right type of map
pub type EntityMap = HashMap<entity::EntityID, Box<entity::Entity>>; 
pub type GSStageFn = fn(Box<dyn GlobalSystem>, EntityMap);

/// Maps from stage to Global System function
pub type GSStageMap = StageMap<GSStageFn>; 

pub type GSFactoryFn = fn() -> Box<dyn GlobalSystem>;

pub trait GlobalSystemDesc {
    const NAME: &'static str;
    const NAME_CRC: u32;
}

/// Empty stage map
pub const EMPTY_STAGE_MAP: GSStageMap = [None; STAGE_COUNT];

/// Generic Data Group Init Arg
pub type GenericGlobalSystemInitArg = Box<dyn GenericGlobalSystemInitArgTrait>;
pub trait GenericGlobalSystemInitArgTrait: CanCast + std::fmt::Debug + Send + Sync {}

pub trait GlobalSystemInitDescTrait {
    /// Arg type, if any
    type ArgType;

    /// Init Description of this GlobalSystem
    const INIT_DESC: InitDesc;
}

/// Similarly to Datagroups, implements the initialization function
pub trait GlobalSystem : ids::HasID + CanCast + std::fmt::Debug + Send + Sync
{
    fn __init__(&mut self, init_data: std::option::Option<Box<dyn GenericGlobalSystemInitArgTrait>>);
}

#[derive(Debug)]
pub struct GlobalSystemRegistryEntry {
    pub id: GlobalSystemID,
    pub name: &'static str,
    pub name_crc: u32,
    pub dependencies: Vec<Dependency>,
    pub functions: GSStageMap,
    pub before: Vec<GlobalSystemID>,
    pub after: Vec<GlobalSystemID>,
    pub factory: GSFactoryFn,
    pub init_desc : InitDesc,
    pub set_id_fn: fn(GlobalSystemID), // Only used for init, don't use it manually
}

#[derive(Debug, Default)]
pub struct GlobalSystemRegistry {
    entries: Vec<GlobalSystemRegistryEntry>,
    is_initialized: bool,
}

impl GlobalSystemRegistry {
    #[inline]
    pub fn new() -> Self {
        GlobalSystemRegistry::default()
    }

    #[inline]
    fn get_temp_global_registry() -> &'static RwLock<TempRegistryLambdas> {
        &LOCAL_SYSTEM_REGISTRY_TEMP
    }

    pub fn register_lambda(lambda: TempRegistryLambda) {
        GlobalSystemRegistry::get_temp_global_registry()
            .write()
            .push(lambda)
    }

    #[inline]
    pub fn get_global_registry() -> &'static RwLock<Self> {
        &GLOBAL_SYSTEM
    }

    pub fn register(&mut self, entry: GlobalSystemRegistryEntry) {
        self.entries.push(entry);
    }

    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Initialize the global registry
    pub fn initialize() {
        let mut registry = GlobalSystemRegistry::get_global_registry().write();
        assert!(
            !registry.is_initialized,
            "Local System registry was already initialized!"
        );

        let mut locals_register_fns = TempRegistryLambdas::new();
        let mut globals_register_fns = GlobalSystemRegistry::get_temp_global_registry().write();

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
    pub fn get_entry_by_id(&self, id: GlobalSystemID) -> &GlobalSystemRegistryEntry {
        debug_assert!((id as usize) < self.entries.len(), "Invalid ID");
        &self.entries[id as usize]
    }

    /// Set ids for local systems based on the topological ordering
    /// generated by the `before` and `after` dependencies. Local systems
    /// can then be sorted by id to get the order in which they should be run
    fn set_toposort_ids(&mut self) {
        let mut ts: TopologicalSort<GlobalSystemID> = TopologicalSort::new();
        let source_node = GlobalSystemID::default();
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
            entry.id = id as GlobalSystemID;
            (entry.set_id_fn)(id as GlobalSystemID);
        }
    }

    /// Get the entry for a specific LocalSystem
    pub fn get_entry<S>(&self) -> &GlobalSystemRegistryEntry
    where
        S: ids::IDLocator + GlobalSystemDesc,
    {
        self.get_entry_by_id(get_id!(S))
    }

    pub fn set_dependencies<S>(&mut self, before: Vec<GlobalSystemID>, after: Vec<GlobalSystemID>)
    where
        S: ids::IDLocator + GlobalSystemDesc,
    {
        // We won't allow changing dependencies in runtime
        debug_assert!(
            !self.is_initialized,
            "You can't set dependencies after initializing local systems"
        );
        let entry = &mut self.entries[get_id!(S) as usize];
        entry.before = before;
        entry.after = after;
    }
}

pub type TempRegistryLambda = Box<dyn FnOnce(&mut GlobalSystemRegistry) + Sync + Send + 'static>;
type TempRegistryLambdas = Vec<TempRegistryLambda>;

lazy_static! {

    // This registry holds functions that register a local system.
    // It's filled before main so that we choose when to call this functions.
    static ref LOCAL_SYSTEM_REGISTRY_TEMP: RwLock<TempRegistryLambdas> =
        RwLock::from(TempRegistryLambdas::new());
}

lazy_static! {
    static ref GLOBAL_SYSTEM: RwLock<GlobalSystemRegistry> =
        RwLock::from(GlobalSystemRegistry::new());
}
