pub mod opengl;
pub mod winit_window;

/// Supported platforms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platforms {
    None,
    Windows,
}
