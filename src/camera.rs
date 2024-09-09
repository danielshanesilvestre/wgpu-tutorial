
pub struct Camera {
    pub eye: glam::Vec3,
    pub center: glam::Vec3,
    pub up: glam::Vec3,
    pub aspect_ratio: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

pub const OPENGL_TO_WGPU_MATRIX: glam::Mat4 = glam::Mat4::from_cols_array(
    &[1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0]
);

impl Camera {
    pub fn build_view_projection_matrix(&self) -> glam::Mat4 {
        let view = glam::Mat4::look_at_rh(self.eye, self.center, self.up);
        
        let proj = glam::Mat4::perspective_rh(
            f32::to_radians(self.fovy),
            self.aspect_ratio,
            self.znear,
            self.zfar
        );

        proj * view
    }
}


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: glam::Mat4,
}