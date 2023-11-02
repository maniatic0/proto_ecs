
#[cfg(test)]
mod sgs
{
    use ecs_macros::CanCast;

    use crate::systems::global_systems::*;

    #[derive(Debug, CanCast)]
    pub struct Test
    {
        _a : u32,
        _b : String
    }

    fn factory() -> Box<dyn GlobalSystem>
    {
        Box::new(Test{_a : 69, _b : "Hello world".to_string()})
    }

    register_global_system!{
        Test,
        factory = factory,
        stages = (42),
        init_arg = OptionalArg(Test)
    }

    impl TestGlobalSystem for Test
    {
        fn init(&mut self, _init_data:std::option::Option<std::boxed::Box<Test>>) {
            
        }

        fn stage_42(&mut self, _entity_map : crate::systems::global_systems::EntityMap) {
            
        }
    }
}