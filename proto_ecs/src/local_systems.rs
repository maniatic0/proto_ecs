pub use ecs_macros::{local_system, register_local_system};
/// Local systems are basically functions that operate on datagroups from
/// an entity. To define a local system, the user should be able to
/// write a function with datagroups it expects as parameters and
/// annotate a macro attribute that will register that system. E.g:
///
/// #[local_system]
/// pub fn MySystem(animation : &mut AnimationDatagroup, mesh : &mut MeshDatagroup)
/// { ... }
use lazy_static::lazy_static;
use parking_lot::RwLock;
use proto_ecs::core::casting::CanCast;
use proto_ecs::data_group::DataGroupID;

use crate::data_group::DataGroup;

pub type SystemClassID = u32;

pub trait CanRun<Args> {
    fn run(&mut self, args: Args);
}

pub trait LocalSystem: LocalSystemMeta + CanCast {
    fn run(datagroups: Box<dyn DataGroup>);
}

pub trait LocalSystemMeta {
    fn get_id(&self) -> SystemClassID;
}

pub type LocalSystemFactory = fn() -> Box<dyn LocalSystem>;

pub type SystemFn = fn(&[usize], &mut Vec<Box<dyn DataGroup>>) -> ();

pub type StageMap = [Option<SystemFn>; 255];

#[derive(Debug, Clone, Copy)]
pub enum Dependency {
    DataGroup(DataGroupID),
    OptionalDG(DataGroupID),
}

impl Dependency {
    pub fn unwrap(self) -> DataGroupID {
        match self {
            Dependency::OptionalDG(d) => d,
            Dependency::DataGroup(d) => d,
        }
    }
}

#[derive(Debug)]
pub struct LocalSystemRegistryEntry {
    pub id: SystemClassID,
    pub name_crc: u32,
    pub dependencies: Vec<Dependency>,
    pub functions: StageMap,
}

#[derive(Debug, Default)]
pub struct LocalSystemRegistry {
    entries: Vec<LocalSystemRegistryEntry>,
    is_initialized: bool,
}

impl LocalSystemRegistry {
    pub fn new() -> Self {
        LocalSystemRegistry::default()
    }

    fn get_temp_global_registry() -> &'static RwLock<TempRegistryLambdas> {
        &LOCAL_SYSTEM_REGISTRY_TEMP
    }

    pub fn register_lambda(lambda: TempRegistryLambda) {
        LocalSystemRegistry::get_temp_global_registry()
            .write()
            .push(lambda)
    }

    pub fn get_global_registry() -> &'static RwLock<Self> {
        &GLOBAL_SYSTEM
    }

    pub fn register_internal(&mut self, mut entry: LocalSystemRegistryEntry) {
        entry.id = self.entries.len() as u32;
        self.entries.push(entry);
    }

    pub fn get_entry_by_id(&self, id: u32) -> &LocalSystemRegistryEntry {
        // TODO Improve this search
        self.entries
            .iter()
            .find(|reg| reg.name_crc == id)
            .expect("Invalid id")
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    pub fn initialize() {
        let mut locals: TempRegistryLambdas = TempRegistryLambdas::new();
        let mut registry = LocalSystemRegistry::get_global_registry().write();
        assert!(
            !registry.is_initialized,
            "Local System registry was already initialized!"
        );

        let mut globals = LocalSystemRegistry::get_temp_global_registry().write();

        // Clear globals
        std::mem::swap(&mut locals, &mut globals);

        // Consume locals
        locals.into_iter().for_each(|lambda| lambda(&mut registry));

        registry.is_initialized = true;
    }
}

pub type TempRegistryLambda = Box<dyn FnOnce(&mut LocalSystemRegistry) + Sync + Send + 'static>;
type TempRegistryLambdas = Vec<TempRegistryLambda>;

lazy_static! {
    static ref LOCAL_SYSTEM_REGISTRY_TEMP: RwLock<TempRegistryLambdas> =
        RwLock::from(TempRegistryLambdas::new());
}

lazy_static! {
    static ref GLOBAL_SYSTEM: RwLock<LocalSystemRegistry> =
        RwLock::from(LocalSystemRegistry::new());
}

/* Example usage of a local system:

pub struct Animation;

local_system!{
    Animation,
    dependencies = [
        AnimationDatagroup,
        MeshDatagroup,
        Optional(ClothDatagroup)
    ],
    stages = [
        Init,
        FrameUpdate,
        127
    ]
}

Now implement this auto generated trait:
impl AnimationLocalSystem for Animation
{
    fn Init(animation_datagroup : AnimationDataGroup, mesh_datagroup : MeshDataGroup, cloth_datagroup : optional<ClothDatagroup>)
    {...}

    fn FrameUpdate(animation_datagroup : AnimationDataGroup, mesh_datagroup : MeshDataGroup)
    {...}

    fn Stage127(animation_datagroup : AnimationDataGroup, mesh_datagroup : MeshDataGroup)
    {...}
}

*/
