
#[cfg(test)]
pub mod sgs
{
    use ecs_macros::CanCast;
    use crate::systems::global_systems::*;
    use crate::tests::shared_datagroups::sdg::{AnimationDataGroup, MeshDataGroup};

    // -- < First global system > ------------------------------
    #[derive(Debug, CanCast)]
    pub struct Test
    {
        pub _a : u32,
        pub _b : String
    }

    fn factory() -> Box<dyn GlobalSystem>
    {
        Box::new(Test{_a : 69, _b : "Hello world".to_string()})
    }

    register_global_system!{
        Test,
        factory = factory,
        stages = (42),
        init_arg = OptionalArg(Test), 
        dependencies = (AnimationDataGroup, MeshDataGroup),
        before = (TestAfter),
        after = (TestBefore)
    }

    impl TestGlobalSystem for Test
    {
        fn init(&mut self, _init_data: Option<Box<Test>>) {
            if _init_data.is_none()
            {
                return;
            }
            let _init_data = _init_data.unwrap();
            self._a = _init_data._a;
            self._b = _init_data._b;
        }

        fn stage_42(&mut self, _entity_map : &crate::systems::global_systems::EntityMap) {
            self._a *= 2;
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
        Box::new(Test{_a : 69, _b : "Before world".to_string()})
    }

    register_global_system!{
        TestBefore,
        factory = factory_before,
        stages = (42),
        init_arg = Arg(TestBefore)
    }

    impl TestBeforeGlobalSystem for TestBefore
    {
        fn init(&mut self, _init_data:std::boxed::Box<TestBefore>) {
            
        }

        fn stage_42(&mut self, _entity_map : &crate::systems::global_systems::EntityMap) {

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
        init_arg = NoArg
    }

    impl TestAfterGlobalSystem for TestAfter
    {
        fn init(&mut self) {
            
        }

        fn stage_42(&mut self, _entity_map : &crate::systems::global_systems::EntityMap) {
            
        }
    }
}