#[cfg(test)]
mod local_system_test {
    use super::super::shared_datagroups::sdg::*;
    use crate::entity_spawn_desc::EntitySpawnDescription;
    use crate::{app::App, core::casting::cast, get_id, local_systems::LocalSystemRegistry};
    use proto_ecs::data_group::*;
    use proto_ecs::local_systems::register_local_system;

    // -- Local system creation
    struct Test;

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

    struct TestOpt;

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

    #[test]
    fn test_local_system_registration() {
        if !App::is_initialized() {
            App::initialize();
        }

        let dg_registry = DataGroupRegistry::get_global_registry().read();
        let ls_registry = LocalSystemRegistry::get_global_registry().read();
        let mesh = dg_registry.create::<MeshDataGroup>();
        let anim = dg_registry.create::<AnimationDataGroup>();
        let mut dgs = vec![anim, mesh];
        let indices: [usize; 2] = [0, 1];
        let entry = ls_registry.get_entry::<Test>();
        for f in entry.functions {
            match f {
                Some(f) => (f)(&indices, &mut dgs),
                _ => {}
            }
        }

        let anim: &AnimationDataGroup = cast(&dgs[0]);
        assert_eq!(
            anim.duration, 4.20,
            "System is not affecting the intended datagroup"
        );
        assert_eq!(
            entry.dependencies.len(),
            2,
            "There should be two dependencies for this system"
        );
        assert!(
            entry.dependencies[0].unwrap() == get_id!(AnimationDataGroup)
                && entry.dependencies[1].unwrap() == get_id!(MeshDataGroup),
            "Inconsistent dependencies for local system"
        )
    }

    #[test]
    fn test_local_system_entity_spawn_desc() {
        if !App::is_initialized() {
            App::initialize();
        }

        {
            // Part 1, everything uninit
            let mut spawn_desc = EntitySpawnDescription::default();

            Test::simple_prepare(&mut spawn_desc);
            assert!(spawn_desc.get_local_system::<Test>());

            assert!(matches!(
                spawn_desc.get_datagroup::<AnimationDataGroup>(),
                Some(DataGroupInitType::Uninitialized(_))
            ));
            assert!(matches!(
                spawn_desc.get_datagroup::<MeshDataGroup>(),
                Some(DataGroupInitType::NoArg)
            ));
        }

        {
            // Part 2, animation datagroup init
            let mut spawn_desc = EntitySpawnDescription::default();
            let init_params = Box::new(AnimationDataGroup {
                clip_name: "hello world".to_string(),
                duration: 4.20,
            });

            AnimationDataGroup::prepare_spawn(&mut spawn_desc, init_params);
            assert!(matches!(
                spawn_desc.get_datagroup::<AnimationDataGroup>(),
                Some(DataGroupInitType::Arg(_))
            ));

            Test::simple_prepare(&mut spawn_desc);
            assert!(spawn_desc.get_local_system::<Test>());

            assert!(matches!(
                spawn_desc.get_datagroup::<AnimationDataGroup>(),
                Some(DataGroupInitType::Arg(_))
            ));
            assert!(matches!(
                spawn_desc.get_datagroup::<MeshDataGroup>(),
                Some(DataGroupInitType::NoArg)
            ));
        }
    }

    #[test]
    fn test_local_system_before_after()
    {
        if !App::is_initialized()
        {
            App::initialize();
        }
        let global_registry = LocalSystemRegistry::get_global_registry().read();
        let entry = global_registry.get_entry::<Test>();

        assert_eq!(entry.before.len(), 1, "Wrong number of `before` dependencies");
        assert_eq!(entry.before[0], get_id!(TestOpt), "Wrong number of `before` dependencies");
    }
}
