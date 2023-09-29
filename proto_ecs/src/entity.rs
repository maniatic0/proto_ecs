use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::data_group::{DataGroupId, DataGroupDyn};
use crate::systems::{SystemClassID, LocalSystemDyn};
use ecs_macros::entity;

pub type EntityClassID = u32;
pub type EntityID = u32;

/// User behavior is implemented here, so that we can create a derive macro for
// the entity while also allowing users to provide their default implementation.
/// Users implement this trait explicitly for their entity struct
trait UserEntityDyn 
{

}

/// All dynamic behavior is implemented in this trait so that instances of this
// trait can be converted to trait objects. This object includes user defined
// behavior.
/// Implement this trait manually
trait EntityDyn : UserEntityDyn
{
    fn get_storage(&self) -> &EntityStorage;
}

/// Autogenerated metadata for an entity type. Don't implement directly from
// this, the implementation should be auto generated via #\[derive] macro.
/// Don't implement directly from this
trait EntityDesc 
{
    fn get_class_id() -> EntityClassID;

    fn get_class_name() -> &'static str;
}

/// This trait represents the overall behavior of an entity, including both
// dynamic and static behavior.
/// Note that there should not be many implementations of this trait, the
// default default implementation provided in this file should be enough.
/// Implemet this trait manually
trait Entity : EntityDyn + EntityDesc
{
    // This is a trait (and not a struct) because we might want to allow custom implementations of 
    // of the entity type if necessary.

    fn factory(datagroups : &Vec<DataGroupId>, systems : &Vec<SystemClassID>) -> Box<dyn EntityDyn>;
}

pub type EntityInstanceFactory = fn(&Vec<DataGroupId>, &Vec<SystemClassID>) -> Box<dyn EntityDyn>;

// -- < Entity Registration > -----------------------------------------------

#[derive(Clone)]
pub struct EntityRegistryEntry
{
    datagroups : Vec<DataGroupId>,
    systems    : Vec<SystemClassID>, 
    name : &'static str,
    id : EntityClassID,
    factory : EntityInstanceFactory
}

/// This struct holds the data for an Entity, local systems and datagroups
pub struct EntityStorage
{
    local_systems : Vec<Box<dyn LocalSystemDyn>>,
    datagroups : Vec<Box<dyn DataGroupDyn>>
}

impl EntityStorage
{

}

pub struct EntityClassRegistry
{
    entity_entries : Vec<EntityRegistryEntry>
}

lazy_static!{
    pub static ref GLOBAL_ENTITY_REGISTRY : Mutex<EntityClassRegistry> = Mutex::from(EntityClassRegistry::new());
}

impl EntityClassRegistry
{
    fn new() -> EntityClassRegistry
    {
        EntityClassRegistry { entity_entries: vec![] }
    }

    fn register(&mut self, description : &EntityRegistryEntry)
    {
        self.entity_entries.push(description.clone());
    }

    fn get_global_registry() -> &'static Mutex<EntityClassRegistry>
    {
        return &GLOBAL_ENTITY_REGISTRY;
    }

    fn create_entity(&self, class_id : EntityClassID) -> Box<dyn EntityDyn>
    {
        let class = self.entity_entries
                                        .iter()
                                        .find(|&register| 
                                            {register.id == class_id}
                                        ).expect(
                                            format!("No entity for such id: {}", class_id).as_str()
                                        );
        
        return (class.factory)(&class.datagroups, &class.systems);
    }
}

macro_rules! register_entity_class {
    ($i:ident, datagroups=[$($d:expr),*], local_systems=[$(s:expr),*]) => {
        const _ : () = {
            #[ctor::ctor]
            fn __register_entity_class__()
            {
                $crate::entity::GLOBAL_REGISTRY
                    .lock()
                    .as_mut()
                    .and_then(
                        | registry |
                        {
                            registry.register(
                                $crate::entity::EntityRegistryEntry{
                                    datagroups : vec![$($d:expr),*],
                                    systems : vec![$($s:expr),*],
                                    name : $i::get_class_name(),
                                    id : $i::get_class_id(),
                                    factory : $i::factory
                                }
                            );

                            return Ok(());
                        }
                    ).expect("This lock should not be poisoned right now")
            }
        };
    };
}

// -- < Default Entity Implementation > -----------------------------

/// This is the default entity implementation. Users should use this struct most
/// of the times when creating an entity, which is specified in a data oriented
/// manner by providing a list of datagroups and local systems for the entity.
/// 
/// Don't create this entity directly, ask the global registry to construct an 
/// entity of the specified type.

#[entity]
pub struct GameEntity
{
    storage : EntityStorage // for local systems and datagroups
}

impl UserEntityDyn for GameEntity
{ }

impl EntityDyn for GameEntity
{
    fn get_storage(&self) -> &EntityStorage {
        return &self.storage;
    }
}

// impl Entity for GameEntity
// {
//     fn factory(datagroups : &Vec<DataGroupId>, systems : &Vec<SystemClassID>) -> Box<dyn EntityDyn> {
        
//     }
// }