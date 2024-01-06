use ecs_macros::register_datagroup;
use proto_ecs::{
    core::casting::CanCast,
    data_group::{DataGroup, GenericDataGroupInitArgTrait},
};

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

impl TestNumberDataGroupDesc for TestNumberDataGroup {
    fn init(&mut self, init_data: std::boxed::Box<TestNumberDataGroupArg>) {
        self.num = init_data.num;
    }
}

register_datagroup!(
    TestNumberDataGroup,
    test_num_factory,
    init_style = Arg(TestNumberDataGroupArg)
);
