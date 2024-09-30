use macaw::Vec3A;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    // TODO I feel like this transform matrix should be a custom type with some helper functions
    position: macaw::Vec3A,
    up_vector: macaw::Vec3A,
    eye_direction: macaw::Vec3A,
    aspect_ratio: f32,
    params: PerspectiveParams,
}

#[derive(Debug, Clone, Copy)]
pub enum PerspectiveParams {
    Ortho(/* TODO */),
    Perspective {
        y_fov_degrees: f32,
        z_far: f32,
        z_near: f32,
    },
}

impl Default for Camera {
    fn default() -> Self {
        Camera::new(
            Vec3A::ZERO,
            Vec3A::new(0.0, 0.0, 1.0),
            Vec3A::new(0.0, -1.0, 0.0),
            16.0 / 9.0,
            PerspectiveParams::Perspective {
                y_fov_degrees: 110.0,
                z_far: 100.0,
                z_near: 5.0,
            },
        )
    }
}

impl Camera {
    pub fn new(
        position: Vec3A,
        up_vector: macaw::Vec3A,
        eye_direction: macaw::Vec3A,
        aspect_ratio: f32,
        perspective: PerspectiveParams,
    ) -> Self {
        // By default, look up (positive z) and forward (negative y)
        Self {
            position,
            up_vector,
            eye_direction,
            params: perspective,
            aspect_ratio,
        }
    }

    /// Creates a perspective matrix to map from view space, to homogeneus clip space
    /// (the canonical view volume with each axis ranging from -1 to 1).
    ///
    /// Returns a transformation matrix: Mv -> Mh
    /// Where v = View Space or Camera Space,
    ///       h = Homogenous Clip Space
    #[inline(always)]
    pub fn perspective_matrix(
        &self,
        fov_y_radians: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> macaw::Mat4 {
        macaw::Mat4::perspective_lh(fov_y_radians, aspect_ratio, z_near, z_far)
    }

    pub fn ortho_matrix(
        &self,
        z_near: f32,
        z_far: f32,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
    ) -> macaw::Mat4 {
        macaw::Mat4::orthographic_lh(left, right, bottom, top, z_near, z_far)
    }

    /// Creates a transformation matrix to map from world to camera space.
    ///
    /// Returns a transformation matrix: Mw -> Mv
    /// Where   w = World space
    ///         v = View space o or Camera Space
    #[inline(always)]
    pub fn world_to_camera_matrix(&self) -> macaw::Mat4 {
        let view_to_world = macaw::Mat4::look_to_lh(
            self.position.into(),
            self.eye_direction.normalize().into(),
            self.up_vector.normalize().into(),
        );
        view_to_world
    }

    #[inline(always)]
    pub fn set_position(&mut self, position: macaw::Vec3A) {
        self.position = position;
    }

    #[inline(always)]
    pub fn set_up_vector(&mut self, new_up: macaw::Vec3A) {
        self.up_vector = new_up.normalize();
    }

    #[inline(always)]
    pub fn look_at(&mut self, target: macaw::Vec3A) {
        self.eye_direction = (target - self.position).normalize();
    }

    #[inline(always)]
    pub fn set_aspect_ratio(&mut self, new_aspect_ratio: f32) {
        self.aspect_ratio = new_aspect_ratio;
    }
}
