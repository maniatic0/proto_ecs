use proto_ecs::systems::common::*;

type GlobalSystemID = u16;
type GSStageMap = StageMap<fn()>; // TODO Not sure what the signature of global system functions is

#[derive(Debug)]
pub struct LocalSystemRegistryEntry {
    pub id: GlobalSystemID,
    pub name: &'static str,
    pub name_crc: u32,
    pub dependencies: Vec<Dependency>,
    pub functions: GSStageMap,
    pub before: Vec<GlobalSystemID>,
    pub after: Vec<GlobalSystemID>,
    pub set_id_fn: fn(GlobalSystemID), // Only used for init, don't use it manually
}

#[derive(Debug, Default)]
pub struct LocalSystemRegistry {
    entries: Vec<LocalSystemRegistryEntry>,
    is_initialized: bool,
}