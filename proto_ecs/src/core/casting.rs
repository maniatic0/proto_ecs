/// This mod implements casting features between our object model types
pub use ecs_macros::CanCast;

use std::any::Any;

pub trait CanCast {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: CanCast + ?Sized> CanCast for Box<T>
where
    T: 'static,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        <T as CanCast>::into_any(*self)
    }

    fn as_any(&self) -> &dyn Any {
        <T as CanCast>::as_any(self)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        <T as CanCast>::as_any_mut(self)
    }
}

#[inline(always)]
pub fn safe_cast<V, T>(v: &V) -> Option<&T>
where
    V: CanCast + ?Sized,
    T: 'static,
{
    v.as_any().downcast_ref::<T>()
}

#[inline(always)]
pub fn safe_cast_mut<V, T>(v: &mut V) -> Option<&mut T>
where
    V: CanCast + ?Sized,
    T: 'static,
{
    v.as_any_mut().downcast_mut::<T>()
}

#[inline(always)]
pub fn cast<V, T>(v: &V) -> &T
where
    V: CanCast + ?Sized,
    T: 'static,
{
    v.as_any()
        .downcast_ref::<T>()
        .expect("Cast is not possible")
}

#[inline(always)]
pub fn cast_mut<V, T>(v: &mut V) -> &mut T
where
    V: CanCast + ?Sized,
    T: 'static,
{
    v.as_any_mut()
        .downcast_mut::<T>()
        .expect("Cast is not possible")
}

#[inline(always)]
pub fn into_any<T>(v: Box<impl CanCast + ?Sized>) -> Box<T>
where
    T: 'static,
{
    v.into_any().downcast().expect("Cast is not possible")
}
