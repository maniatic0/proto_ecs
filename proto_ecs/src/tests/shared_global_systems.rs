#[cfg(test)]
pub mod sgs {
    use crate::data_group::{DataGroup, GenericDataGroupInitArgTrait};
    use crate::systems::global_systems::*;
    use crate::tests::shared_datagroups::sdg::{AnimationDataGroup, MeshDataGroup};
    use ecs_macros::{CanCast, register_datagroup, register_datagroup_init};
    use crate::entities::entity_system::*;

    // -- < First global system > ------------------------------
    #[derive(Debug, CanCast)]
    pub struct Test {
        pub _a: u32,
        pub _b: String,
    }

    fn factory() -> Box<dyn GlobalSystem> {
        Box::new(Test {
            _a: 69,
            _b: "Hello world".to_string(),
        })
    }

    register_global_system! {
        Test,
        factory = factory,
        stages = (42),
        init_arg = OptionalArg(Test),
        dependencies = (AnimationDataGroup, MeshDataGroup),
        before = (TestAfter),
        after = (TestBefore)
    }

    impl TestGlobalSystem for Test {
        fn init(&mut self, _init_data: Option<Box<Test>>) {
            if _init_data.is_none() {
                return;
            }
            let _init_data = _init_data.unwrap();
            self._a = _init_data._a;
            self._b = _init_data._b;
        }

        fn stage_42(&mut self, world : &World, _entity_map: &crate::entities::entity_system::EntityMap, _registered_entities: &Vec<proto_ecs::entities::entity_system::EntityPtr>) {
            self._a *= 2;
        }
    }

    // -- < Second global system > ------------------------------
    #[derive(Debug, CanCast)]
    pub struct TestBefore {
        _a: u32,
        _b: String,
    }

    fn factory_before() -> Box<dyn GlobalSystem> {
        Box::new(Test {
            _a: 69,
            _b: "Before world".to_string(),
        })
    }

    register_global_system! {
        TestBefore,
        factory = factory_before,
        stages = (42),
        init_arg = Arg(TestBefore)
    }

    impl TestBeforeGlobalSystem for TestBefore {
        fn init(&mut self, _init_data: std::boxed::Box<TestBefore>) {}

        fn stage_42(&mut self, world : &World, _entity_map: &crate::entities::entity_system::EntityMap, _registered_entities: &Vec<proto_ecs::entities::entity_system::EntityPtr>) {
            
        }
    }

    // -- < Third global system > ------------------------------
    #[derive(Debug, CanCast)]
    pub struct TestAfter {
        _a: u32,
        _b: String,
    }

    fn factory_after() -> Box<dyn GlobalSystem> {
        Box::new(Test {
            _a: 68,
            _b: "after world".to_string(),
        })
    }

    register_global_system! {
        TestAfter,
        factory = factory_after,
        stages = (42),
        init_arg = NoArg
    }

    impl TestAfterGlobalSystem for TestAfter {
        fn init(&mut self) {}

        fn stage_42(&mut self, world : &World, _entity_map: &crate::entities::entity_system::EntityMap, _registered_entities: &Vec<proto_ecs::entities::entity_system::EntityPtr>) {
            
        }
    }

    /// This datagroup is used along with GSFlowTester 
    /// global system to check the global system flow
    #[derive(CanCast, Debug)]
    pub struct GSFlowDG {
        pub id: usize,
    }

    impl GenericDataGroupInitArgTrait for GSFlowDG {}

    fn gs_flow_factory() -> Box<dyn DataGroup> {
        return Box::new(GSFlowDG {
            id: 0,
        });
    }

    register_datagroup_init!(GSFlowDG, NoArg);

    impl GSFlowDGDesc for GSFlowDG {
        fn init(&mut self) {
            self.id = 0;
        }
    }

    register_datagroup!(GSFlowDG, gs_flow_factory);

    #[derive(Debug, CanCast)]
    pub struct GSFlowTester 
    {
        pub n_entities : usize,
    }

    fn gs_flow_tester_factory() -> Box<dyn GlobalSystem>
    {
        return Box::new(GSFlowTester{n_entities: 0});
    }

    register_global_system!{
        GSFlowTester,
        factory = gs_flow_tester_factory,
        stages = (69),
        dependencies = (GSFlowDG)
    }

    impl GSFlowTesterGlobalSystem for GSFlowTester
    {
        
        fn stage_69(&mut self, world : &World, entity_map: &crate::entities::entity_system::EntityMap,registered_entities: &Vec<crate::entities::entity_system::EntityPtr>) {
            self.n_entities = registered_entities.len();
            for (i, entity) in registered_entities.iter().enumerate()
            {
                let mut entity_ptr = entity.write();
                let dg = entity_ptr.get_datagroup_mut::<GSFlowDG>().unwrap();
                dg.id = i+1;
            }
        }
        
        
    }
}
