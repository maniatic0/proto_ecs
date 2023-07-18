use std::any::Any;

use crc32fast;

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
    fn get_name_crc() -> u32;

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
    fn get_name_crc(&self) -> u32;

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
    type InitParams;
    fn init(&mut self, params: Self::InitParams);
}

/// Factory function to create default Data Groups
pub type DataGroupFactory = fn() -> Box<dyn DataGroupDyn>;

pub struct DataGroupRegistryEntry {
    pub name: &'static str,
    pub name_crc: u32,
    pub factory_func: DataGroupFactory,
}

pub struct DataGroupRegistry {
    entries: Vec<DataGroupRegistryEntry>,
}
