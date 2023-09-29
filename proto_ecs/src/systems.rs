
pub type SystemClassID = u32;


pub trait UserLocalSystem
{

}

pub trait LocalSystemDyn : UserLocalSystem {
    
}

pub trait LocalSystem : LocalSystemDyn
{

}