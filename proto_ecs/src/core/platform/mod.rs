pub mod winit_window;
pub mod opengl;

/// Supported platforms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platforms {
    None, 
    Windows
}