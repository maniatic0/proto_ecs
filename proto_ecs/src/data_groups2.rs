// This is an alternative version of the Datagroup API. 
// This is more data oriented in the sense that many of the 
// system-accounting things are implemented as a strictly and static-ly
// defined struct, while the user can describe its custom datagroup with a 
// factory function and a struct that implements the datagroup interface.
// 
// The user-implemented bits are usually resolved using 
// dynamic dispatch and are defined by the user. The user code will know
// nothing about the resource management part of the engine, which will mostly 
// use static dispatching.

use std::any::Any;
use lazy_static::lazy_static;
use std::sync::Mutex;

pub type DatagroupID = u32;

/// This wrapper is the system required part of the datagroup. The user defined and 
/// implemented part is unknown at compile time, so we store a pointer to the 
/// actual datagroup
pub struct DataGroupWrapper
{
    name : &'static str,
    name_crc : u32,
    id : u32,
    datagroup : Box<dyn DataGroup>
}

pub trait DataGroupInitParams 
{
    fn as_any(&self) -> &dyn Any;
}

/// This trait is the user implementable part of a datagroup.
/// Users will create a DataGroup and register it.
/// 
/// Every function that requires data from the wrapper containing the datagroup
/// will receive the wrapper as an argument.
/// 
/// Example usage 
/// 
/// ```rust
/// pub struct MyDatagroup {
///     ...
/// }
/// 
/// impl DataGroup for MyDatagroup {
///     Explicit implementation of Datagroup
/// }
/// 
/// pub fn factory() -> Box<dyn DataGroup>
/// {
///     return Box::from(MyDatagroup::new())
/// }
/// 
/// register_datagroup!(MyDatagroup, factory)
/// ```
pub trait DataGroup 
{
    fn init_data(&mut self, init_data : Box<dyn DataGroupInitParams>);
}

/// Factory function to create default Data Groups
pub type DataGroupFactory = fn() -> Box<dyn DataGroup>;

/// Entry for the datagroup Registry
/// 
/// Specifies the data describing a specific datagroup
#[derive(Debug)]
pub struct DataGroupRegistryEntry 
{
    pub name: &'static str,
    pub name_crc: u32,
    pub factory_func: DataGroupFactory,
}

/// Datagroup Registry used to store and manage datagroups
/// 
/// There should be just one instance of this registry in the entire application, 
/// accessible through a static method
#[derive(Debug)]
pub struct DataGroupRegistry 
{
    entries: Vec<DataGroupRegistryEntry>,
}

impl DataGroupRegistry
{
    /// Create a new empty registry
    pub fn new() -> DataGroupRegistry
    {
        DataGroupRegistry { entries: vec![] }
    }

    /// Create a registry from a list of entries
    pub fn from_entries(entries : Vec<DataGroupRegistryEntry>) -> DataGroupRegistry
    {
        DataGroupRegistry{entries}
    }

    ///  Add a new entry to the registry
    pub fn register(&mut self, entry : DataGroupRegistryEntry)
    {
        self.entries.push(entry);
    }

    pub fn load_registered_datagroups() -> DataGroupRegistry
    {
        unimplemented!("not yet implemented");
    }

    /// Get the global registry.
    /// 
    /// This is the registry used by default to gather all structs registered
    /// with the register_datagroup! macro
    pub fn get_global_registry() -> &'static Mutex<DataGroupRegistry>
    {
        return &GLOBAL_REGISTRY;
    }
}

lazy_static!{
    /// This registry holds entries for all datagroups registered in this application
    pub static ref GLOBAL_REGISTRY : Mutex<DataGroupRegistry> = Mutex::from(DataGroupRegistry::new());
}
