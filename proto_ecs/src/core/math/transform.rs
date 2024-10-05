use macaw::{self, EulerRot, Quat};

/// Helper data structure to represent transformations
#[derive(Copy, Clone)]
pub struct TransformMatrix {
    /// Internal matrix representation
    matrix: macaw::Affine3A,
}

impl TransformMatrix {
    /// Get the transformation matrix corresponding to this transform
    #[inline(always)]
    pub fn matrix(&self) -> macaw::Affine3A {
        self.matrix
    }

    pub fn translate(&mut self, translation: macaw::Vec3A) {
        self.matrix.translation += translation;
    }

    pub fn rotate(&mut self, euler_degs: macaw::Vec3A) {
        let old_translation = self.matrix.translation;
        self.matrix.translation = macaw::Vec3A::ZERO;
        self.matrix = macaw::Affine3A::from_rotation_translation(
            Quat::from_euler(EulerRot::XYZ, euler_degs.x, euler_degs.y, euler_degs.z),
            macaw::Vec3::ZERO,
        ) * self.matrix;
        self.matrix.translation = old_translation;
    }

    pub fn scale(&mut self, scale : macaw::Vec3) {
        self.matrix *=  macaw::Affine3A::from_scale(scale);
    }
}
