/// This mod implements casting features between our object model types
pub use ecs_macros::CanCast;

pub trait CanCast {
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

#[inline(always)]
pub fn safe_cast<V, T>(v: &Box<V>) -> Option<&T>
where
    V: CanCast + ?Sized,
    T: 'static,
{
    v.as_any().downcast_ref::<T>()
}

#[inline(always)]
pub fn safe_cast_mut<V, T>(v: &mut Box<V>) -> Option<&mut T>
where
    V: CanCast + ?Sized,
    T: 'static,
{
    v.as_any_mut().downcast_mut::<T>()
}

#[inline(always)]
pub fn cast<V, T>(v: &Box<V>) -> &T
where
    V: CanCast + ?Sized,
    T: 'static,
{
    v.as_any()
        .downcast_ref::<T>()
        .expect("Cast is not possible")
}

#[inline(always)]
pub fn cast_mut<V, T>(v: &mut Box<V>) -> &mut T
where
    V: CanCast + ?Sized,
    T: 'static,
{
    v.as_any_mut()
        .downcast_mut::<T>()
        .expect("Cast is not possible")
}

#[inline(always)]
pub fn into_any<V, T>(v: Box<V>) -> Box<T>
where
    V: CanCast + ?Sized,
    T: 'static,
{
    v.into_any().downcast().expect("Cast is not possible")
}
