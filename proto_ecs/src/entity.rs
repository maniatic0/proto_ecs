use crate::data_group::DataGroupId;
use crate::systems::SystemClassID;

// All dynamic behavior is implemented here so that instances of this trait can be converted 
// to trait objects
trait EntityDyn 
{

}

trait Entity : EntityDyn 
{
    // This is a trait (and not a struct) because we might want to allow custom implementations of 
    // of the entity type if necessary.
    fn from_description(description : &dyn EntityDescription) -> Box<dyn EntityDyn>;
}

// ? I think this will be auto implemented by a sophisticated macro call instead of manually 
// ? implementing this. 
trait EntityDescription
{
    fn get_datagroups(&self) -> Vec<DataGroupId>;
    fn get_local_systems(&self) -> Vec<SystemClassID>;
}
