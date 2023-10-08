/// This mod implements casting features between our object model types
pub use ecs_macros::CanCast;

pub trait CanCast
{
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
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

pub fn cast_t<T>(v : &Box<dyn CanCast>) -> &T
    where T : 'static
{
    v.as_any().downcast_ref::<T>().expect("Cast is not possible")
}

pub fn into_any<T>(v : Box<dyn CanCast>) -> Box<T>
    where T : 'static
{
    v.into_any().downcast().expect("Cast is not possible")
}