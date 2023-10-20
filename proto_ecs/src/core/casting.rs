/// This mod implements casting features between our object model types
pub use ecs_macros::CanCast;

pub trait CanCast {
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

#[inline(always)]
pub fn safe_cast<T>(v: &Box<impl CanCast + ?Sized>) -> Option<&T>
where
    T: 'static,
{
    v.as_any().downcast_ref::<T>()
}

#[inline(always)]
pub fn safe_cast_mut<T>(v: &mut Box<impl CanCast + ?Sized>) -> Option<&mut T>
where
    T: 'static,
{
    v.as_any_mut().downcast_mut::<T>()
}

#[inline(always)]
pub fn cast<T>(v: &Box<impl CanCast + ?Sized>) -> &T
where
    T: 'static,
{
    v.as_any()
        .downcast_ref::<T>()
        .expect("Cast is not possible")
}

#[inline(always)]
pub fn cast_mut<T>(v: &mut Box<impl CanCast + ?Sized>) -> &mut T
where
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
