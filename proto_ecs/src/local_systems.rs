/// Local systems are basically functions that operate on datagroups from
/// an entity. To define a local system, the user should be able to
/// write a function with datagroups it expects as parameters and
/// annotate a macro attribute that will register that system. E.g:
///
/// #[local_system]
/// pub fn MySystem(animation : &mut AnimationDatagroup, mesh : &mut MeshDatagroup)
/// { ... }
use lazy_static::lazy_static;
use parking_lot::RwLock;
use proto_ecs::core::casting::CanCast;
use proto_ecs::core::ids;
use proto_ecs::data_group::DataGroupID;
use proto_ecs::get_id;
use topological_sort::TopologicalSort;
use crate::data_group::DataGroup;
pub use ecs_macros::register_local_system;


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

pub type SystemFn = fn(&[usize], &mut Vec<Box<dyn DataGroup>>) -> ();

// BEGIN TODO: Move this to be shared with global systems as well (?)

pub type StageID = u8;

/// Number of stages supported by the engine
pub const STAGE_COUNT: usize = StageID::MAX as usize + 1;

/// Stage Map type
pub type StageMap = [Option<SystemFn>; STAGE_COUNT];

/// Empty stage map
pub const EMPTY_STAGE_MAP: StageMap = [None; STAGE_COUNT];

// END TODO: Move this to be shared with global systems as well (?)

pub trait LocalSystemDesc 
{
    const NAME : &'static str;
    const NAME_CRC : u32;
}

#[derive(Debug, Clone, Copy)]
pub enum Dependency {
    DataGroup(DataGroupID),
    OptionalDG(DataGroupID),
}

impl Dependency {
    pub fn unwrap(self) -> DataGroupID {
        match self {
            Dependency::OptionalDG(d) => d,
            Dependency::DataGroup(d) => d,
        }
    }
}

#[derive(Debug)]
pub struct LocalSystemRegistryEntry {
    pub id: SystemClassID,
    pub name: &'static str,
    pub name_crc: u32,
    pub dependencies: Vec<Dependency>,
    pub functions: StageMap,
    pub before: Vec<SystemClassID>,
    pub after: Vec<SystemClassID>,
    pub set_id_fn : fn(SystemClassID) 
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

    #[inline]
    fn get_temp_global_dependency_registry() -> &'static RwLock<TempRegistryLambdas> {
        &LOCAL_SYSTEM_DEPENDENCY_REGISTRY_TEMP
    }

    pub fn register_lambda(lambda: TempRegistryLambda) {
        LocalSystemRegistry::get_temp_global_registry()
            .write()
            .push(lambda)
    }

    pub fn register_dependency_lambda(lambda: TempRegistryLambda) {
        LocalSystemRegistry::get_temp_global_dependency_registry()
            .write()
            .push(lambda)
    }

    #[inline]
    pub fn get_global_registry() -> &'static RwLock<Self> {
        &GLOBAL_SYSTEM
    }

    pub fn register(&mut self, entry: LocalSystemRegistryEntry){
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

        let mut locals_dep_register_fns = TempRegistryLambdas::new();
        let mut globals_dep_register_fns =
            LocalSystemRegistry::get_temp_global_dependency_registry().write();

        // Clear globals
        std::mem::swap(&mut locals_register_fns, &mut globals_register_fns);
        std::mem::swap(&mut locals_dep_register_fns, &mut globals_dep_register_fns);

        registry.init(locals_register_fns, locals_dep_register_fns);
    }

    /// Initialize this registry entry
    pub fn init(
        &mut self,
        registry_fns: TempRegistryLambdas,
        dependency_registry_fns: TempRegistryLambdas,
    ) {
        registry_fns.into_iter().for_each(|lambda| lambda(self));
        self.set_toposort_ids();

        self.entries
            .sort_unstable_by(|this, other| this.id.cmp(&other.id));
        dependency_registry_fns
            .into_iter()
            .for_each(|lambda| lambda(self));

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
    fn set_toposort_ids(&mut self)
    {
        let mut ts : TopologicalSort<SystemClassID> = TopologicalSort::new();
        let source_node = SystemClassID::default();
        for entry in self.entries.iter()
        {
            let entry_crc = entry.name_crc;
            ts.add_dependency(source_node, entry_crc);

            // Sanity check
            debug_assert!(
                source_node != entry.name_crc, 
                "Source node should be a value never reachable by the crc"
            );
            for &other_crc in entry.before.iter()
            {
                ts.add_dependency(entry_crc, other_crc);
            }

            for &other_crc in entry.after.iter()
            {
                ts.add_dependency(other_crc, entry_crc);
            }
        }

        let source_node_vec = ts.pop_all();
        debug_assert!(source_node_vec.len() == 1, "The first dependency should be only the source node");
        debug_assert!(source_node_vec[0] == source_node, "The first dependency should be the source node");
        let mut dependency_order = vec![];
        while ts.len() > 0
        {
            let mut non_dependents = ts.pop_all();
            if non_dependents.len() == 0 && ts.len() != 0
            {
                // If there's cyclic dependencies, then the popped list is empty
                // and ts.len > 0, 
                // See: https://docs.rs/topological-sort/latest/topological_sort/struct.TopologicalSort.html#method.pop_all
                // TODO: better error handling
                panic!("Cyclic dependencies between local systems!");
            }
            
            // Non-dependents are elements that do not depend on anything else.
            // Sort them by value to get a deterministic ordering each time.
            // Since they don't depend on each other, the actual order doesn't matter.
            non_dependents.sort();
            dependency_order.extend(non_dependents);
        }

        for entry in self.entries.iter_mut()
        {
            let id = dependency_order.iter().position(|&crc| entry.name_crc == crc).unwrap();
            entry.id = id as SystemClassID;
            (entry.set_id_fn)(id as SystemClassID);
        }

    }

    /// Get the entry for a specific LocalSystem
    pub fn get_entry<S>(&self) -> &LocalSystemRegistryEntry
    where
        S: ids::IDLocator, // TODO Add local system trait here if we decide we need one
    {
        self.get_entry_by_id(get_id!(S))
    }

    pub fn set_dependencies<S>(&mut self, before: Vec<SystemClassID>, after: Vec<SystemClassID>)
    where
        S: ids::IDLocator,
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

    // This registry holds functions that register dependencies for a local system.
    // Note that this functions depend on local systems being loaded and sorted, so call
    // this functions only if you are sure that's the case.
    static ref LOCAL_SYSTEM_DEPENDENCY_REGISTRY_TEMP : RwLock<TempRegistryLambdas> =
        RwLock::from(TempRegistryLambdas::new());
}

lazy_static! {
    static ref GLOBAL_SYSTEM: RwLock<LocalSystemRegistry> =
        RwLock::from(LocalSystemRegistry::new());
}