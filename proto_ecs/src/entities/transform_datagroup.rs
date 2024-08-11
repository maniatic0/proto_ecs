use bitvec::view::BitViewSized;
use ecs_macros::{register_datagroup, CanCast};
/// A Transform datagroup that represents the spatial information about an entity and
/// its spatial relationships to other entities.
///
/// This is the only datagroup allowed to have references to other entities,
/// and those reference are strictly controlled.
///
/// Users should not have access to this datagroup.
use std::sync::atomic::AtomicUsize;

use crate::{
    data_group::{DataGroup, GenericDataGroupInitArgTrait},
    entities::entity_allocator::EntityPtr,
    systems::common::STAGE_COUNT,
};

/// Vector used to represent transform's positions
pub type TransformPosition = macaw::Vec3;

/// Vector used to represent transform's scales
pub type TransformScale = macaw::Vec3;

/// Vector used to represent transform's rotations
pub type TransformRotation = macaw::Quat;

/// Matrix type used by the transform datagroup
pub type TransformMatrix = macaw::Affine3A;

/// A spatial hierarchy for entities. Entities that provide this
/// datagroup can define spatial relationships to other entities.
#[derive(Debug, CanCast)]
pub struct Transform {
    pub(super) parent: Option<EntityPtr>,
    pub(super) children: Vec<EntityPtr>,

    /// How many nodes in this spatial hierarchy, including the current node.
    pub(super) n_nodes: usize, // ? Should this be an Atomic instead?

    /// Amount of entities to run per stage in this hierarchy,
    /// including the current node
    pub(super) stage_count: [AtomicUsize; STAGE_COUNT],

    /// Cached Parent World Transform
    cached_parent_world_transform: TransformMatrix,

    /// Cached Inverse Parent World Transform
    cached_inverse_parent_world_transform: TransformMatrix,

    /// World Position for this entity
    cached_world_position: TransformPosition,

    /// Local Position
    local_position: TransformPosition,

    /// Local Rotation
    local_rotation: TransformRotation,

    /// Local Scale
    local_scale: TransformScale,
}

impl GenericDataGroupInitArgTrait for Transform {}
register_datagroup!(Transform, factory, init_style = Arg(Transform));

impl TransformDesc for Transform {
    fn init(&mut self, init_data: Box<Transform>) {
        self.children = init_data.children;
        self.n_nodes = init_data.n_nodes;
        self.parent = init_data.parent;
        self.stage_count = init_data.stage_count;
        self.cached_parent_world_transform = init_data.cached_parent_world_transform;
        self.cached_inverse_parent_world_transform =
            init_data.cached_inverse_parent_world_transform;
        self.cached_world_position = init_data.cached_world_position;
        self.local_position = init_data.local_position;
        self.local_rotation = init_data.local_rotation;
        self.local_scale = init_data.local_scale;
    }
}

impl Transform {
    /// Checks if this hierarchy node is the root of some hierarchy
    #[inline(always)]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Set the local transform matrix
    #[inline(always)]
    pub fn get_local_transform_mat(&self) -> TransformMatrix {
        TransformMatrix::from_scale_rotation_translation(
            self.local_scale,
            self.local_rotation,
            self.local_position,
        )
    }

    /// Set the local transform position
    #[inline(always)]
    pub fn set_local_position(&mut self, new_position: TransformPosition) {
        self.local_position = new_position;
        self.cached_world_position = self
            .cached_parent_world_transform
            .transform_point3(new_position)
    }

    /// Set the local transform rotation
    #[inline(always)]
    pub fn set_local_rotation(&mut self, new_rotation: TransformRotation) {
        self.local_rotation = new_rotation
    }

    /// Set the local transform scale
    #[inline(always)]
    pub fn set_local_scale(&mut self, new_scale: TransformScale) {
        self.local_scale = new_scale
    }

    /// Get local transform position
    #[inline(always)]
    pub fn get_local_position(&self) -> &TransformPosition {
        &self.local_position
    }

    /// Get local transform rotation
    #[inline(always)]
    pub fn get_local_rotation(&self) -> &TransformRotation {
        &self.local_rotation
    }

    /// Get local transform scale
    #[inline(always)]
    pub fn get_local_scale(&self) -> &TransformScale {
        &self.local_scale
    }

    /// Set the world transform position
    #[inline(always)]
    pub fn set_world_position(&mut self, new_position: TransformPosition) {
        self.local_position = self
            .cached_inverse_parent_world_transform
            .transform_point3(new_position);
        self.cached_world_position = new_position
    }

    /// Get world transform position
    pub fn get_world_positon(&self) -> &TransformPosition {
        &self.cached_world_position
    }

    /// Set the parent transform matrix
    #[inline(always)]
    pub(super) fn set_parent_transform_mat(&mut self, new_transform_mat: TransformMatrix) {
        self.cached_parent_world_transform = new_transform_mat;
        self.cached_inverse_parent_world_transform = self.cached_parent_world_transform.inverse();

        // Update current world position from our new parent position
        self.cached_world_position = self
            .cached_parent_world_transform
            .transform_point3(self.local_position)
    }

    /// Get the parent transform matrix
    #[inline(always)]
    pub fn get_parent_transform_mat(&self) -> &TransformMatrix {
        &self.cached_parent_world_transform
    }

    pub fn get_world_transform_mat(&self) -> TransformMatrix {
        *self.get_parent_transform_mat() * self.get_local_transform_mat()
    }
}

fn factory() -> Box<dyn DataGroup> {
    Box::<Transform>::default()
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            n_nodes: 1,
            parent: None,
            children: vec![],
            stage_count: std::array::from_fn(|_| AtomicUsize::ZERO),
            cached_parent_world_transform: TransformMatrix::IDENTITY,
            cached_inverse_parent_world_transform: TransformMatrix::IDENTITY,
            local_position: TransformPosition::ZERO,
            local_scale: TransformScale::ONE,
            local_rotation: TransformRotation::IDENTITY,
            cached_world_position: TransformPosition::ZERO,
        }
    }
}
