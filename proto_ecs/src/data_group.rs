// This is an alternative version of the Datagroup API.
// This is more data oriented in the sense that many of the
// system-accounting things are implemented as a strictly and static-ly
// defined struct, while the user can describe its custom datagroup with a
// factory function and a struct that implements the datagroup interface.
//
// The user-implemented bits are usually resolved using
// dynamic dispatch and are defined by the user. The user code will know
// nothing about the resource management part of the engine, which will mostly
// use static dispatching. To access this information, user-defined datagroups
// can use their datagroup id as key with the global registry to get its own data

pub use ecs_macros::register_datagroup;
use lazy_static::lazy_static;
use proto_ecs::core::casting::CanCast;
use proto_ecs::core::{ids, locking::RwLock};
use proto_ecs::get_id;
use std::fmt::Debug;

use crate::core::common::InitDesc;

pub type DataGroupID = u32;

/// Generic trait for DataGroup Init Args
pub trait GenericDataGroupInitArgTrait: CanCast + Debug + Send + Sync {}

/// Generic Data Group Init Arg
pub type GenericDataGroupInitArg = Box<dyn GenericDataGroupInitArgTrait>;

#[derive(Debug)]
/// Whether a DataGroup has an init function
/// If it has one, it can specify if it doesn't take an argument,
/// if the argument is required, or if the argument is optional
pub enum DataGroupInitType {
    /// Uninitialized Arg (Owner info)
    Uninitialized(&'static str),
    /// Datagroup without init
    NoInit,
    /// Datagroup with init but no args
    NoArg,
    /// Datagroup with init and args
    Arg(GenericDataGroupInitArg),
    /// Datagroup with init and optional args
    OptionalArg(Option<GenericDataGroupInitArg>),
}

/// This trait is the user implementable part of a datagroup.
/// Users will create a DataGroup and register it with a macro to be
/// available for construction.
///
/// Every function that requires data from the wrapper containing the datagroup
/// will receive the wrapper as an argument.
///
/// Example usage
///
/// ```ignore
/// use proto_ecs::data_group::{DataGroup, register_datagroup_v2, DataGroupInitParams};
/// pub struct MyDatagroup {
///     
/// }
///
/// impl DataGroup for MyDatagroup {
///     fn init(&mut self, init_data : Box<dyn DataGroupInitParams>)
///     { }
/// }
///
/// pub fn factory() -> Box<dyn DataGroup>
/// {
///     return Box::from(MyDatagroup{})
/// }
///
/// register_datagroup!(MyDatagroup, factory)
/// ```
pub trait DataGroup: ids::HasID + CanCast + std::fmt::Debug + Send + Sync {
    fn __init__(&mut self, init_data: std::option::Option<Box<dyn GenericDataGroupInitArgTrait>>);
}

/// Trait to implement the description of the init step for a datagroup
/// This is automatically implemented by macros
pub trait DataGroupInitDescTrait {
    /// Arg type, if any
    type ArgType;

    /// Init Description of this DataGroup
    const INIT_DESC: InitDesc;
}

/// Factory function to create default Data Groups
pub type DataGroupFactory = fn() -> Box<dyn DataGroup>;

/// Datagroup's static description
pub trait DatagroupDesc {
    /// Name of this datagroup
    const NAME: &'static str;
    /// Name's crc
    const NAME_CRC: u32;
    /// Factory to create new instances of this datagroup
    const FACTORY: DataGroupFactory;
}

/// Entry for the datagroup Registry
///
/// Specifies the data describing a specific datagroup
#[derive(Debug)]
pub struct DataGroupRegistryEntry {
    pub name: &'static str,
    pub name_crc: u32,
    pub factory_func: DataGroupFactory,
    pub init_desc: InitDesc,
    pub id: DataGroupID,
}

lazy_static! {
    /// This registry holds entries for all datagroups registered in this application
    pub static ref GLOBAL_REGISTRY : RwLock<DataGroupRegistry> = RwLock::from(DataGroupRegistry::new());
}

/// Datagroup Registry used to store and manage datagroups
///
/// There should be just one instance of this registry in the entire application,
/// accessible through a static method
#[derive(Debug, Default)]
pub struct DataGroupRegistry {
    entries: Vec<DataGroupRegistryEntry>,
    is_initialized: bool,
}

impl DataGroupRegistry {
    /// Call this first thing before running game play code.
    pub fn init(&mut self) {
        assert!(
            !self.is_initialized,
            "Data Group Registry got double initialized!"
        );
        self.entries
            .sort_by(|entry1, entry2| entry1.id.cmp(&entry2.id));
        self.is_initialized = true;
    }

    /// Initialize global registry
    pub fn initialize() {
        let mut locals: TempRegistryLambdas = TempRegistryLambdas::new();
        let mut registry = DataGroupRegistry::get_global_registry().write();
        assert!(
            !registry.is_initialized,
            "Local System registry was already initialized!"
        );

        let mut globals = DataGroupRegistry::get_temp_global_registry().write();

        // Clear globals
        std::mem::swap(&mut locals, &mut globals);

        // Consume locals
        locals.into_iter().for_each(|lambda| lambda(&mut registry));

        registry.init();
    }

    #[inline]
    /// Create a new empty registry
    pub fn new() -> DataGroupRegistry {
        Default::default()
    }

    #[inline]
    /// Create a registry from a list of entries
    pub fn from_entries(entries: Vec<DataGroupRegistryEntry>) -> DataGroupRegistry {
        DataGroupRegistry {
            entries,
            ..Default::default()
        }
    }

    #[inline(always)]
    /// If the data group registry is initialized
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    #[inline]
    ///  Add a new entry to the registry
    pub fn register(&mut self, mut entry: DataGroupRegistryEntry) -> DataGroupID {
        let new_id = self.entries.len() as u32;
        entry.id = new_id;
        self.entries.push(entry);
        new_id
    }

    pub fn load_registered_datagroups() -> DataGroupRegistry {
        unimplemented!("not yet implemented");
    }

    #[inline(always)]
    /// Get the global registry.
    ///
    /// This is the registry used by default to gather all structs registered
    /// with the register_datagroup! macro
    pub fn get_global_registry() -> &'static RwLock<DataGroupRegistry> {
        &GLOBAL_REGISTRY
    }

    #[inline]
    pub fn get_entry_by_id(&self, id: DataGroupID) -> &DataGroupRegistryEntry {
        debug_assert!((id as usize) < self.entries.len(), "Invalid id");
        &self.entries[id as usize]
    }

    #[inline(always)]
    pub fn get_entry<D>(&self) -> &DataGroupRegistryEntry
    where
        D: ids::IDLocator + DataGroup,
    {
        self.get_entry_by_id(get_id!(D))
    }

    #[inline]
    pub fn create_by_id(&self, id: DataGroupID) -> Box<dyn DataGroup> {
        let entry = self.get_entry_by_id(id);
        (entry.factory_func)()
    }

    #[inline(always)]
    pub fn create<D>(&self) -> Box<dyn DataGroup>
    where
        D: ids::IDLocator + DataGroup,
    {
        self.create_by_id(get_id!(D))
    }

    pub fn register_lambda(lambda: TempRegistryLambda) {
        DataGroupRegistry::get_temp_global_registry()
            .write()
            .push(lambda);
    }

    pub fn get_temp_global_registry() -> &'static RwLock<TempRegistryLambdas> {
        &DATAGROUP_REGISTRY_TEMP
    }
}

#[macro_export]
/// Create a new datagroup registered in the global registry.
macro_rules! create_datagroup {
    ($dg:ident) => {{
        let global_registry =
            proto_ecs::data_group::DataGroupRegistry::get_global_registry().read();
        global_registry.create::<$dg>()
    }};
}

// Implement into iter so you can iterate over the registry entries:
// for entry in entries.iter()
// { ... }
impl<'a> IntoIterator for &'a DataGroupRegistry {
    type Item = &'a DataGroupRegistryEntry;
    type IntoIter = std::slice::Iter<'a, DataGroupRegistryEntry>;

    fn into_iter(self) -> Self::IntoIter {
        return self.entries.iter();
    }
}

// Use This registry to store datagroup initialization functions
pub type TempRegistryLambda = Box<dyn FnOnce(&mut DataGroupRegistry) + Sync + Send + 'static>;
type TempRegistryLambdas = Vec<TempRegistryLambda>;

lazy_static! {
    static ref DATAGROUP_REGISTRY_TEMP: RwLock<TempRegistryLambdas> =
        RwLock::from(TempRegistryLambdas::new());
}
