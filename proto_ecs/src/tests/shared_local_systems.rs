#[cfg(test)]
pub mod sls {
    use crate::tests::shared_datagroups::sdg::{
        AnimationDataGroup, MeshDataGroup, TestNumberDataGroup,
    };
    use proto_ecs::entities::entity::EntityID;
    use proto_ecs::local_systems::register_local_system;

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
            _entity_id: EntityID,
            animation_data_group: &mut AnimationDataGroup,
            _mesh_data_group: &mut MeshDataGroup,
        ) {
            animation_data_group.duration = 4.2;
        }

        fn stage_1(
            _entity_id: EntityID,
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
            _entity_id: EntityID,
            _animation_data_group: &mut AnimationDataGroup,
            _mesh_data_group: Option<&mut MeshDataGroup>,
        ) {
        }
    }

    pub struct TestAdder;

    register_local_system! {
        TestAdder,
        dependencies = (TestNumberDataGroup),
        stages = (0)
    }

    impl TestAdderLocalSystem for TestAdder {
        fn stage_0(_entity_id: EntityID, test_number_data_group: &mut TestNumberDataGroup) {
            test_number_data_group.num = test_number_data_group.num + 1
        }
    }

    pub struct TestMultiplier;

    register_local_system! {
        TestMultiplier,
        dependencies = (TestNumberDataGroup),
        stages = (0),
        after = (TestAdder)
    }

    impl TestMultiplierLocalSystem for TestMultiplier {
        fn stage_0(_entity_id: EntityID, test_number_data_group: &mut TestNumberDataGroup) {
            test_number_data_group.num = test_number_data_group.num * 2
        }
    }
}
