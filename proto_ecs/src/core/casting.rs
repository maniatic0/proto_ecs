/// This mod implements casting features between our object model types
pub use ecs_macros::CanCast;

pub trait CanCast
{
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) ->&mut dyn std::any::Any;
}

#[macro_export]
macro_rules! cast {
    ($v:expr, $t:ident) => {
        ($v).as_any().downcast_ref::<$t>().expect("Cast is not possible")
    };
}

#[macro_export]
macro_rules! cast_mut {
    ($v:expr, $t:ident) => {
        ($v).as_any_mut().downcast_mut::<$t>().expect("Cast is not possible")
    };
}

#[macro_export]
macro_rules! safe_cast {
    ($v:expr, $t:ident) => {
        ($v).as_any().downcast_ref::<$t>()
    };
}

#[macro_export]
macro_rules! safe_cast_mut {
    ($v:expr, $t:ident) => {
        ($v).as_any_mut().downcast_mut::<$t>()
    };
}