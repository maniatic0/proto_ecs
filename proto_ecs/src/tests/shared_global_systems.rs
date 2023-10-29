
#[cfg(test)]
mod sgs
{
    use ecs_macros::CanCast;

    use crate::systems::global_systems::*;

    #[derive(Debug, CanCast)]
    pub struct TestGlobalSystem
    {
        a : u32,
        b : String
    }

    fn factory() -> Box<dyn GlobalSystem>
    {
        Box::new(TestGlobalSystem{a : 69, b : "Hello world".to_string()})
    }

    register_global_system!{
        TestGlobalSystem,
        factory = factory
    }
}