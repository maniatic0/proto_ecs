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

use lazy_static::lazy_static;
use parking_lot::RwLock;
use proto_ecs::core::casting::CanCast;
pub use ecs_macros::register_datagroup;

pub type DataGroupID = u32;

/// This trait it's a little hack to get the id from any dyn DataGroup instance.
/// Don't implement directly from this, it will be implemented by the register_datagroup macro
pub trait DataGroupMeta 
{
    fn get_id(&self) -> DataGroupID;
}

/// This trait represents compile time metadata about datagroups. Is implemented
/// by the registry per datagroup. It's implemented automagically with the 
/// register_datagroup macro
pub trait DataGroupMetadataLocator
{
    fn get_id() -> DataGroupID;
}

/// return the ID of the given datagroup
#[macro_export]
macro_rules! get_id {
    ($i:ident) => {
        <$i as proto_ecs::data_group::DataGroupMetadataLocator>::get_id()
    };
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
pub trait DataGroup : DataGroupMeta + CanCast
{
    fn init(&mut self, init_data : Box<dyn CanCast>);
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
    pub static ref GLOBAL_REGISTRY : RwLock<DataGroupRegistry> = RwLock::from(DataGroupRegistry::new());
}

/// Datagroup Registry used to store and manage datagroups
/// 
/// There should be just one instance of this registry in the entire application, 
/// accessible through a static method
#[derive(Debug, Default)]
pub struct DataGroupRegistry 
{
    entries: Vec<DataGroupRegistryEntry>,
    is_initialized: bool
}

impl DataGroupRegistry
{
    /// Call this first thing before running game play code.
    pub fn init(&mut self)
    {
        assert!(!self.is_initialized, "Data Group Registry got double initialized!");
        self.entries
            .sort_by(
                |entry1, entry2| 
                { entry1.id.cmp(&entry2.id) }
            );
        
        self.is_initialized = true;
    }

    /// Create a new empty registry
    pub fn new() -> DataGroupRegistry
    {
        Default::default()
    }

    /// Create a registry from a list of entries
    pub fn from_entries(entries : Vec<DataGroupRegistryEntry>) -> DataGroupRegistry
    {
        DataGroupRegistry{entries, ..Default::default()}
    }

    pub fn is_initialized(&self) -> bool
    {
        self.is_initialized
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
    pub fn get_global_registry() -> &'static RwLock<DataGroupRegistry>
    {
        return &GLOBAL_REGISTRY;
    }

    pub fn get_entry_of(&self, id : DataGroupID) -> &DataGroupRegistryEntry
    {
        assert!((id as usize) < self.entries.len(), "Invalid id");
        return &self.entries[id as usize];
    }

    pub fn get_entry<D>(&self) -> &DataGroupRegistryEntry
        where D : DataGroupMetadataLocator
    {
        self.get_entry_of(get_id!(D))
    }

    pub fn create(&self, id : DataGroupID) -> Box<dyn DataGroup>
    {
        let entry = self.get_entry_of(id);
        
        return (entry.factory_func)();
    }
}

#[macro_export]
/// Create a new datagroup registered in the global registry. 
macro_rules! create_datagroup {
    ($dg:ident) => {
        { 
            let id = <$dg as proto_ecs::data_group::DataGroupMetadataLocator>::get_id();
            let global_registry = proto_ecs::data_group::DataGroupRegistry::get_global_registry().read();
            let entry = global_registry.get_entry_of(id);
            (entry.factory_func)()
         }
    };
}

// Implement into iter so you can iterate over the registry entries:
// for entry in entries.iter()
// { ... }
impl<'a> IntoIterator for &'a DataGroupRegistry
{
    type Item = &'a DataGroupRegistryEntry;
    type IntoIter = std::slice::Iter<'a, DataGroupRegistryEntry>;

    fn into_iter(self) -> Self::IntoIter {
        return self.entries.iter();
    }
}