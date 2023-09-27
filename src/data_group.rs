use std::any::Any;
use crc32fast;
pub use ecs_macros::{datagroup, DataGroupInitParamsDyn};
use lazy_static::lazy_static;
use std::sync::Mutex;
pub use u32 as DataGroupId;

/// Params used during initialization of Data Groups (Dynamic Trait Version)
///
/// They are passed as part of the creation process
/// 
/// Don't implement directly from this
pub trait DataGroupInitParamsDyn {

    /// For casting purposes
    fn as_any(&self) -> &dyn Any;
}


/// Description of a Data Group
///
/// Called components on the original Kruger ECS
pub trait DataGroupDesc {
    /// Unique Name of the Data Group
    fn get_name() -> &'static str;

    /// Name CRC32 (for long term storage)
    fn get_name_crc() -> u32
    {
        crc32fast::hash(Self::get_name().as_bytes())
    }

    /// DataGroup ID generated during engine init (short term storage)
    fn get_data_group_id() -> DataGroupId;
}

/// A group of related data (Dynamic Trait version)
///
/// Cannot contain references to other DataGroups
///
/// Called components on the original Kruger ECS
/// 
/// Don't implement directly from this
pub trait DataGroupDyn {
    /// Unique Name of the Data Group
    fn get_name(&self) -> &'static str;

    /// Name CRC32 (for long term storage)
    fn get_name_crc(&self) -> u32
    {
        crc32fast::hash(self.get_name().as_bytes())
    }

    /// DataGroup ID generated during engine init (short term storage)
    fn get_data_group_id(&self) -> DataGroupId;

    /// Initialize data group based on init Params
    fn dyn_init(&mut self, params: Box<dyn DataGroupInitParamsDyn>);

    /// For casting purposes
    fn as_any(&self) -> &dyn Any;
}

/// A group of related data
///
/// Cannot contain references to other DataGroups
///
/// Called components on the original Kruger ECS
pub trait DataGroup: DataGroupDesc + DataGroupDyn + Default {
    type InitParams : DataGroupInitParamsDyn;
    fn init(&mut self, params: Self::InitParams);

    fn factory() -> Box<dyn DataGroupDyn>;
}

/// Factory function to create default Data Groups
pub type DataGroupFactory = fn() -> Box<dyn DataGroupDyn>;

#[derive(Debug)]
pub struct DataGroupRegistryEntry {
    pub name: &'static str,
    pub name_crc: u32,
    pub factory_func: DataGroupFactory,
}

#[derive(Debug)]
pub struct DataGroupRegistry {
    entries: Vec<DataGroupRegistryEntry>,
}

impl DataGroupRegistry
{
    pub fn new() -> DataGroupRegistry
    {
        DataGroupRegistry { entries: vec![] }
    }

    pub fn from_entries(entries : Vec<DataGroupRegistryEntry>) -> DataGroupRegistry
    {
        DataGroupRegistry{entries}
    }

    pub fn add(&mut self, entry : DataGroupRegistryEntry)
    {
        self.entries.push(entry);
    }

    pub fn load_registered_datagroups() -> DataGroupRegistry
    {
        unimplemented!("Function not yet implemented");
    }

    pub fn get_global_registry() -> &'static Mutex<DataGroupRegistry>
    {
        return &GLOBAL_REGISTRY;
    }
}

lazy_static!{
    /// This registry holds entries for all datagroups registered in this application
    pub static ref GLOBAL_REGISTRY : Mutex<DataGroupRegistry> = Mutex::from(DataGroupRegistry::new());
}

#[macro_export]
macro_rules! register_component {
    ($i:ident) => {
        const _ : () = {
            #[ctor::ctor]
            fn __register_component__()
            {
                $crate::data_group::GLOBAL_REGISTRY.lock().as_mut().and_then(|registry| {
                    registry.add(
                        $crate::data_group::DataGroupRegistryEntry { 
                            name: <$i as $crate::data_group::DataGroupDesc>::get_name(), 
                            name_crc: <$i as $crate::data_group::DataGroupDesc>::get_name_crc(), 
                            factory_func: <$i as $crate::data_group::DataGroup>::factory 
                        });

                    return Ok(());
                }).expect("Could not get lock to register a new component");
            }
        };
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