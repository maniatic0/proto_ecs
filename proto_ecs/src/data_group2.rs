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
pub use ecs_macros::{register_datagroup_v2, DataGroupInitParams};

pub type DataGroupID = u32;


pub trait DataGroupInitParams 
{
    fn as_any(&self) -> &dyn Any;
}

pub trait DataGroupMeta 
{
    fn get_id(&self) -> DataGroupID;
}

/// This trait is the user implementable part of a datagroup.
/// Users will create a DataGroup and register it.
/// 
/// Every function that requires data from the wrapper containing the datagroup
/// will receive the wrapper as an argument.
/// 
/// Example usage 
/// 
/// ```ignore
/// use proto_ecs::data_group2::{DataGroup, register_datagroup_v2, DataGroupInitParams};
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
/// register_datagroup_v2!(MyDatagroup, factory)
/// ```
pub trait DataGroup : DataGroupMeta
{
    fn init(&mut self, init_data : Box<dyn DataGroupInitParams>);
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
    pub id: DataGroupID
}

lazy_static!{
    /// This registry holds entries for all datagroups registered in this application
    pub static ref GLOBAL_REGISTRY : Mutex<DataGroupRegistry> = Mutex::from(DataGroupRegistry::new());
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

    pub fn get_entry_of(&self, id : DataGroupID) -> &DataGroupRegistryEntry
    {
        // TODO we have to optimize search. Since IDs are sequential, we can just sort them and 
        // TODO use direct indexing 
        self.entries
            .iter()
            .find(
                    |entry| 
                    {entry.id == id}
                )
            .expect("Invalid id")
    }

    pub fn create(&self, id : DataGroupID) -> Box<dyn DataGroup>
    {
        let entry = self.get_entry_of(id);
        
        return (entry.factory_func)();
    }

}


/// This trait represents compile time metadata about datagroups. Is implemented
/// by the registry per datagroup. It's implemented automagically with the 
/// register_datagroup macro
pub trait DataGroupMetadataLocator<T : DataGroup>
{
    fn get_id() -> DataGroupID;
}

#[macro_export]
macro_rules! get_id {
    ($i:ident) => {
        <proto_ecs::data_group2::DataGroupRegistry as proto_ecs::data_group2::DataGroupMetadataLocator<$i>>::get_id()
    };
}

// Implement into iter so you can iterate over the registry entries:
// for entry in &entries
// { ... }
impl<'a> IntoIterator for &'a DataGroupRegistry
{
    type Item = &'a DataGroupRegistryEntry;
    type IntoIter = std::slice::Iter<'a, DataGroupRegistryEntry>;

    fn into_iter(self) -> Self::IntoIter {
        return self.entries.iter();
    }
}