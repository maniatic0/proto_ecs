#[cfg(test)]
mod test {
    use bitvec::slice::BitSliceIndex;

    use crate::{
        app::App,
        core::ids::{HasID, IDLocator},
        core::casting::cast,
        entities::{
            entity::Entity,
            entity_spawn_desc::EntitySpawnDescription,
            entity_system::{EntitySystem, World, DEFAULT_WORLD}, self,
        },
        tests::{
            shared_datagroups::sdg::{
                AnimationDataGroup, MeshDataGroup, TestNumberDataGroup, TestNumberDataGroupArg,
            },
            shared_global_systems::sgs::Test as gs_Test,
            shared_global_systems::sgs::{TestBefore, GSFlowTester, GSFlowDG},
            shared_local_systems::sls::{Test, TestAdder, TestAssertNumber4, TestMultiplier},
        }, get_id,
    };

    #[test]
    fn test_entity_creation() {
        if !App::is_initialized() {
            App::initialize();
        }

        let mut spawn_desc = EntitySpawnDescription::default();
        let init_params = Box::new(AnimationDataGroup {
            clip_name: "hello world".to_string(),
            duration: 4.20,
        });

        AnimationDataGroup::prepare_spawn(&mut spawn_desc, init_params);
        MeshDataGroup::prepare_spawn(&mut spawn_desc);
        Test::simple_prepare(&mut spawn_desc);
        gs_Test::simple_prepare(&mut spawn_desc);

        spawn_desc.check_local_systems_panic();

        spawn_desc.set_name("Test Name".to_owned());

        let entity = Entity::init(1, spawn_desc);
        assert_eq!(entity.get_id(), 1);
        assert_eq!(entity.get_name(), "Test Name");

        assert!(matches!(
            entity.get_datagroup::<AnimationDataGroup>(),
            Some(dg) if dg.get_id() == <AnimationDataGroup as IDLocator>::get_id()
        ));
        assert!(matches!(
            entity.get_datagroup::<MeshDataGroup>(),
            Some(dg) if dg.get_id() == <MeshDataGroup as IDLocator>::get_id()
        ));

        assert!(entity.contains_local_system::<Test>());
        assert!(entity.contains_global_system::<gs_Test>());
        // This GS wasn't added, it should crash when returning true 
        assert!(!entity.contains_global_system::<TestBefore>()); 
    }

    #[test]
    fn test_entity_stage_run() {
        if !App::is_initialized() {
            App::initialize();
        }

        let world = World::new(0);

        let mut spawn_desc = EntitySpawnDescription::default();
        let init_params = Box::new(TestNumberDataGroupArg { num: 1 });

        TestNumberDataGroup::prepare_spawn(&mut spawn_desc, init_params);
        TestAdder::simple_prepare(&mut spawn_desc);
        TestMultiplier::simple_prepare(&mut spawn_desc);
        spawn_desc.check_local_systems_panic();

        spawn_desc.set_name("Test Name".to_owned());

        let mut entity = Entity::init(1, spawn_desc);

        entity.run_stage(&world, 0);
        assert_eq!(
            entity.get_datagroup::<TestNumberDataGroup>().unwrap().num,
            4
        );
    }

    #[test]
    fn test_entity_system_basic() {
        if !App::is_initialized() {
            App::initialize();
        }

        let es = EntitySystem::get();
        es.reset(); // In case other tests happened
        es.step(0.0, 0.0); // Process reset

        for _ in 0..100 {
            let mut spawn_desc = EntitySpawnDescription::default();
            let init_params = Box::new(TestNumberDataGroupArg { num: 1 });

            TestNumberDataGroup::prepare_spawn(&mut spawn_desc, init_params);
            TestAdder::simple_prepare(&mut spawn_desc);
            TestMultiplier::simple_prepare(&mut spawn_desc);
            TestAssertNumber4::simple_prepare(&mut spawn_desc);
            spawn_desc.check_local_systems_panic();

            spawn_desc.set_name("Test Name".to_owned());

            es.create_entity(DEFAULT_WORLD, spawn_desc)
                .expect("Failed to create entity!");
        }

        // Create a weird entity to test the global system run
        let mut spawn_desc = EntitySpawnDescription::default();
        GSFlowDG::prepare_spawn(&mut spawn_desc);
        GSFlowTester::simple_prepare(&mut spawn_desc);
        spawn_desc.check_datagroups_panic(); // Should not panic
        spawn_desc.set_name("GSFlow Entity".to_owned());

        let entity_id = es.create_entity(DEFAULT_WORLD, spawn_desc).expect("Failed to create entity!");

        es.step(0.0, 0.0);

        // Check that the entity with `GSFlowDG` has the right state
        let world = es.get_worlds().get(&DEFAULT_WORLD).unwrap();
        let entities = world.get_entities();
        let entity_lock = entities.get(&entity_id).expect("Should retrieve entity that was previously created"); 
        let entity = entity_lock.read();
        let gs_flow_dg = entity.get_datagroup::<GSFlowDG>().expect("This entity should be created with this datagroup");
        assert_eq!(gs_flow_dg.id, 1, "GSFlowTester Global system didn't run properly");
        
        // Check that the state of the `GSFlowTester` is as expected
        let global_systems_lock = world.get_global_systems().read();
        let gs_storage_lock = global_systems_lock
                [get_id!(GSFlowTester) as usize]
                .as_ref()
                .expect("This global system should have storage loaded right").read();

        let gs_storage: &GSFlowTester = cast(&*gs_storage_lock);
        assert_eq!(gs_storage.n_entities, 1);
    }
}
