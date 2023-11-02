
#[cfg(test)]
mod sgs
{
    use ecs_macros::CanCast;
    use crate::systems::global_systems::*;

    // -- < First global system > ------------------------------
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

    // -- < Second global system > ------------------------------
    #[derive(Debug, CanCast)]
    pub struct TestBefore
    {
        _a : u32,
        _b : String
    }

    fn factory_before() -> Box<dyn GlobalSystem>
    {
        Box::new(Test{_a : 68, _b : "Before world".to_string()})
    }

    register_global_system!{
        TestBefore,
        factory = factory_before,
        stages = (42),
        init_arg = OptionalArg(TestBefore)
    }

    impl TestBeforeGlobalSystem for TestBefore
    {
        fn init(&mut self, _init_data:std::option::Option<std::boxed::Box<TestBefore>>) {
            
        }

        fn stage_42(&mut self, _entity_map : crate::systems::global_systems::EntityMap) {
            
        }
    }

    // -- < Third global system > ------------------------------
    #[derive(Debug, CanCast)]
    pub struct TestAfter
    {
        _a : u32,
        _b : String
    }

    fn factory_after() -> Box<dyn GlobalSystem>
    {
        Box::new(Test{_a : 68, _b : "after world".to_string()})
    }

    register_global_system!{
        TestAfter,
        factory = factory_after,
        stages = (42),
        init_arg = OptionalArg(TestAfter)
    }

    impl TestAfterGlobalSystem for TestAfter
    {
        fn init(&mut self, _init_data:std::option::Option<std::boxed::Box<TestAfter>>) {
            
        }

        fn stage_42(&mut self, _entity_map : crate::systems::global_systems::EntityMap) {
            
        }
    }
}