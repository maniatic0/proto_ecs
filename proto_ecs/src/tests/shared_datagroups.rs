#[cfg(test)]
pub mod sdg {
    use proto_ecs::data_group::*;

    use crate::core::casting::CanCast;
    // -- first example datagroup
    #[derive(CanCast, Debug)]
    pub struct AnimationDataGroup {
        pub clip_name: String,
        pub duration: f64,
    }

    register_datagroup!(
        AnimationDataGroup,
        animation_factory,
        init_style = Arg(AnimationDataGroup)
    );

    impl GenericDataGroupInitArgTrait for AnimationDataGroup {}

    fn animation_factory() -> Box<dyn DataGroup> {
        return Box::new(AnimationDataGroup {
            clip_name: "Hello world".to_string(),
            duration: 12.4,
        });
    }

    impl AnimationDataGroupDesc for AnimationDataGroup {
        fn init(&mut self, init_data: Box<AnimationDataGroup>) {
            self.clip_name = init_data.clip_name;
            self.duration = init_data.duration;
        }
    }

    // -- Second example datagroup

    #[derive(CanCast, Debug)]
    pub struct MeshDataGroup {}

    fn mesh_factory() -> Box<dyn DataGroup> {
        return Box::new(MeshDataGroup {});
    }

    register_datagroup!(MeshDataGroup, mesh_factory, init_style = NoArg);

    impl MeshDataGroupDesc for MeshDataGroup {
        fn init(&mut self) {}
    }

    #[derive(CanCast, Default, Debug)]
    pub struct TestNumberDataGroup {
        pub num: u32,
    }

    #[derive(CanCast, Default, Debug)]
    pub struct TestNumberDataGroupArg {
        pub num: u32,
    }

    impl GenericDataGroupInitArgTrait for TestNumberDataGroupArg {}

    fn test_num_factory() -> Box<dyn DataGroup> {
        return Box::new(TestNumberDataGroup::default());
    }

    register_datagroup!(
        TestNumberDataGroup,
        test_num_factory,
        init_style = Arg(TestNumberDataGroupArg)
    );

    impl TestNumberDataGroupDesc for TestNumberDataGroup {
        fn init(&mut self, init_data: std::boxed::Box<TestNumberDataGroupArg>) {
            self.num = init_data.num;
        }
    }
}
