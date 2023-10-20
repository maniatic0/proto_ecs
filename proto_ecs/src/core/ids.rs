pub type ID = u32;
pub use once_cell::sync::OnceCell;

// Helper trait to find the id according to a corresponding class
pub trait IDLocator {
    fn get_id() -> ID;
}

// Helper trait to find the id in a trait object
pub trait HasID {
    fn get_id(&self) -> ID;
}

#[macro_export]
macro_rules! get_id {
    ($i:ident) => {
        <$i as proto_ecs::core::ids::IDLocator>::get_id()
    };
}
