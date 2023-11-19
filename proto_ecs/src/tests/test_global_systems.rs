#[cfg(test)]
mod global_system_test {
    use crate::app::App;
    use crate::core::casting::cast_mut;
    use crate::entities::entity_spawn_desc::EntitySpawnDescription;
    use crate::get_id;
    use crate::systems::global_systems::{EntityMap, GlobalSystemRegistry};
    use crate::tests::shared_datagroups::sdg::{AnimationDataGroup, MeshDataGroup};
    use crate::tests::shared_global_systems::sgs::{Test, TestAfter, TestBefore};

    #[test]
    fn test_global_system_registration() {
        if !App::is_initialized() {
            App::initialize();
        }
        let gs_registry = GlobalSystemRegistry::get_global_registry().read();

        let test_entry = gs_registry.get_entry::<Test>();
        let before_entry = gs_registry.get_entry::<TestBefore>();
        let after_entry = gs_registry.get_entry::<TestAfter>();

        assert_eq!(test_entry.id, get_id!(Test));
        assert_eq!(before_entry.id, get_id!(TestBefore));
        assert_eq!(after_entry.id, get_id!(TestAfter));

        for (i, f) in test_entry.functions.iter().enumerate() {
            assert!(
                (i != 42 && f.is_none()) || (i == 42 && f.is_some()),
                "Missing registered function"
            );
        }

        assert!(
            get_id!(Test) > get_id!(TestBefore),
            "Toposort error: Test GS should run before TestBefore"
        );
        assert!(
            get_id!(Test) < get_id!(TestAfter),
            "Toposort error: Test GS should run after TestAfter"
        );
    }

    #[test]
    fn test_global_system_initialization() {
        if !App::is_initialized() {
            App::initialize();
        }

        let gs_registry = GlobalSystemRegistry::get_global_registry().read();

        {
            // Test that state remains the same when initializing without args
            let mut test_gs = gs_registry.create::<Test>();
            test_gs.__init__(None);
            let test_gs: &mut Test = cast_mut(&mut test_gs);
            assert_eq!(test_gs._a, 69);
            assert_eq!(test_gs._b, "Hello world".to_string());
        }

        {
            let mut test_gs = gs_registry.create::<Test>();
            test_gs.__init__(Some(Box::new(Test {
                _a: 42,
                _b: "foo".to_string(),
            })));
            let test_gs: &mut Test = cast_mut(&mut test_gs);
            assert_eq!(test_gs._a, 42);
            assert_eq!(test_gs._b, "foo".to_string());
        }
    }

    #[test]
    fn test_global_system_run() {
        if !App::is_initialized() {
            App::initialize();
        }

        let gs_registry = GlobalSystemRegistry::get_global_registry().read();

        let mut test_gs = gs_registry.create::<Test>();
        let test_gs_entry = gs_registry.get_entry::<Test>();
        let entity_map = EntityMap::new();

        for f in test_gs_entry.functions {
            match f {
                Some(f) => (f)(&mut test_gs, &entity_map),
                _ => {}
            }
        }

        let test_gs: &mut Test = cast_mut(&mut test_gs);
        assert_eq!(test_gs._a, 69 * 2);
    }

    #[test]
    #[should_panic]
    fn test_simple_prepare_should_panic() {
        // check that you can register a global system with simple prepare and
        // that checks panics when they should
        let mut spawn_desc = EntitySpawnDescription::default();
        Test::simple_prepare(&mut spawn_desc);
        spawn_desc.check_panic();
    }

    #[test]
    fn test_simple_prepare_should_not_panic() {
        // check that you can register a global system with simple prepare and
        // that checks panics when they should
        let mut spawn_desc = EntitySpawnDescription::default();
        MeshDataGroup::prepare_spawn(&mut spawn_desc);
        AnimationDataGroup::prepare_spawn(
            &mut spawn_desc,
            Box::new(AnimationDataGroup {
                clip_name: "hello clip".to_string(),
                duration: 42.0,
            }),
        );
        Test::simple_prepare(&mut spawn_desc);
        spawn_desc.check_panic();
    }
}
