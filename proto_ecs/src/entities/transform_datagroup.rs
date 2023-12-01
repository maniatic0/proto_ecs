/// A Transform datagroup that represents the spatial information about an entity and
/// its spatial relationships to other entities. 
/// 
/// This is the only datagroup allowed to have references to other entities, 
/// and those reference are strictly controlled. 
/// 
/// Users should not have access to this datagroup.
use std::sync::atomic::AtomicUsize;
use bitvec::view::BitViewSized;
use ecs_macros::{CanCast, register_datagroup, register_datagroup_init};

use crate::{entities::entity_allocator::EntityPtr, systems::common::STAGE_COUNT, data_group::{DataGroup, GenericDataGroupInitArgTrait}};

/// A spatial hierarchy for entities. Entities that provide this 
/// datagroup can define spatial relationships to other entities.
#[derive(Debug, CanCast)]
pub struct Transform
{
    pub(super) parent : Option<EntityPtr>,
    pub(super) children : Vec<EntityPtr>,

    /// How many nodes in this spatial hierarchy, including the current node.
    pub(super) n_nodes : usize, // ? Should this be an Atomic instead?

    /// Amount of entities to run per stage in this hierarchy, 
    /// including the current node
    pub(super) stage_count : [AtomicUsize; STAGE_COUNT]
}

impl GenericDataGroupInitArgTrait for Transform {}
register_datagroup!(Transform, factory);
register_datagroup_init!(Transform, Arg(Transform));

impl TransformDesc for Transform
{
    fn init(&mut self, init_data : Box<Transform>) {
        self.children = init_data.children;
        self.n_nodes = init_data.n_nodes;
        self.parent = init_data.parent;
        self.stage_count = init_data.stage_count;
    }
}

impl Transform {

    /// Checks if this hierarchy node is the root of some hierarchy
    #[inline(always)]
    pub fn is_root(&self) -> bool
    {
        self.parent.is_none()
    }
}

fn factory() -> Box<dyn DataGroup>{
    Box::new(Transform::default())
}

impl Default for Transform
{
    fn default() -> Self {
        Self{
            n_nodes: 1,
            parent: None, 
            children: vec![],
            stage_count: std::array::from_fn(|_| {AtomicUsize::ZERO})
        }
    }
}