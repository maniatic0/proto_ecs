#[cfg(test)]
pub mod sls {
    use proto_ecs::local_systems::register_local_system;

    use crate::tests::shared_datagroups::sdg::{AnimationDataGroup, MeshDataGroup};

    // -- Local system creation
    pub struct Test;

    register_local_system! {
        Test,
        dependencies = (AnimationDataGroup, MeshDataGroup),
        stages = (0, 1),
        before = (TestOpt)
    }

    impl TestLocalSystem for Test {
        fn stage_0(
            animation_data_group: &mut AnimationDataGroup,
            _mesh_data_group: &mut MeshDataGroup,
        ) {
            animation_data_group.duration = 4.2;
        }

        fn stage_1(
            _animation_data_group: &mut AnimationDataGroup,
            _mesh_data_group: &mut MeshDataGroup,
        ) {
        }
    }

    pub struct TestOpt;

    register_local_system! {
        TestOpt,
        dependencies = (AnimationDataGroup, Optional(MeshDataGroup)),
        stages = (0)
    }

    impl TestOptLocalSystem for TestOpt {
        fn stage_0(
            _animation_data_group: &mut AnimationDataGroup,
            _mesh_data_group: Option<&mut MeshDataGroup>,
        ) {
        }
    }
}