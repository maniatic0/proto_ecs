/// This mod implements casting features between our object model types
pub use ecs_macros::CanCast;

pub trait CanCast
{
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) ->&mut dyn std::any::Any;
}