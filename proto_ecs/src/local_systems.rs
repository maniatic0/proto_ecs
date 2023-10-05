/// Local systems are basically functions that operate on datagroups from 
/// an entity. To define a local system, the user should be able to 
/// write a function with datagroups it expects as parameters and 
/// annotate a macro attribute that will register that system. E.g:
/// 
/// #[local_system]
/// pub fn MySystem(animation : &mut AnimationDatagroup, mesh : &mut MeshDatagroup)
/// { ... }
use crc32fast;
use std::collections::HashMap;
use proto_ecs::data_group::DataGroupID;
use proto_ecs::core::casting::CanCast;

use crate::data_group::DataGroup;

pub type SystemClassID = u32;

pub trait CanRun<Args>
{
    fn run(&mut self, args : Args);
}

pub trait LocalSystem : LocalSystemMeta + CanCast
{
    fn run(datagroups : Box<dyn DataGroup>);
}

pub trait LocalSystemMeta
{
    fn get_id(&self) -> SystemClassID;
}

pub type LocalSystemFactory = fn () -> Box<dyn LocalSystem>;

pub struct LocalSystemRegistryEntry
{
    id : SystemClassID, 
    name_crc : u32,
    dependencies : Vec<DataGroupID>,
    func : fn(Vec<Box<dyn DataGroup>>) -> ()
}

pub struct LocalSystemRegistry
{
    entries : Vec<LocalSystemRegistryEntry>
}

// -------------------------------------------------
// Example of resulting implementation of a local system


pub fn my_local_system()
{

}

