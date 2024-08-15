

/// Handles for resources like buffers and shaders.
/// We use a concrete type to ensure that resource handles are always of the 
/// same type no matter the backend
#[derive(Debug, Clone, Copy)]
pub struct Handle {
    pub(super) index : u32,
    pub(super) generation : u32
}
