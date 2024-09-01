use crate::entities::transform_datagroup::TransformMatrix;
use glam::Affine3A;
use macaw::{Quat, Vec3A};

pub struct Camera {
    // TODO I feel like this transform matrix should be a custom type with some helper functions
    position: macaw::Vec3A,
    up_vector: macaw::Vec3A,
    eye_direction: macaw::Vec3A,
}

/// Parameters to define a view matrix, either a perspective matrix
/// or an orthographic perspective matrix.
pub struct ViewMatrixParams {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
    near: f32,
    far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::new(
            Vec3A::ZERO,
            Vec3A::new(0.0, 0.0, 1.0),
            Vec3A::new(0.0, -1.0, 0.0),
        )
    }
}

impl Camera {
    pub fn new(position: Vec3A, up_vector: macaw::Vec3A, eye_direction: macaw::Vec3A) -> Self {
        // By default, look up (positive z) and forward (negative y)
        Self {
            position,
            up_vector,
            eye_direction,
        }
    }

    /// Creates a perspective matrix to map from view space, to homogeneus clip space 
    /// (the canonical view volume with each axis ranging from -1 to 1).
    /// 
    /// Returns a transformation matrix: Mv -> Mh
    /// Where v = View Space or Camera Space,
    ///       h = Homogenous Clip Space
    #[inline(always)]
    pub fn perspective_matrix(&self, fov_y_radians : f32, aspect_ratio : f32, z_near : f32, z_far : f32) -> macaw::Mat4 {
        macaw::Mat4::perspective_lh(fov_y_radians, aspect_ratio, z_near, z_far)
    }

    /// Creates a transformation matrix to map from world to camera space. 
    /// 
    /// Returns a transformation matrix: Mw -> Mv
    /// Where   w = World space
    ///         v = View space o or Camera Space
    #[inline(always)]
    pub fn world_to_camera_matrix(&self) -> macaw::Mat4 {
        let view_to_world = macaw::Mat4::look_to_lh(self.position.into(), self.eye_direction.into(), self.up_vector.into());
        view_to_world.inverse()
    }
}
