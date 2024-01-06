#[cfg(test)]
mod test {
    use std::sync::atomic::Ordering;

    use bitvec::store::BitStore;

    use crate::{
        app::App,
        core::casting::cast,
        core::ids::{HasID, IDLocator},
        entities::{
            entity_allocator::EntityAllocator,
            entity_spawn_desc::EntitySpawnDescription,
            entity_system::{EntitySystem, World},
            transform_datagroup::Transform,
        },
        get_id,
        systems::common::STAGE_COUNT,
        tests::{
            shared_datagroups::sdg::{
                AnimationDataGroup, MeshDataGroup, TestNumberDataGroup, TestNumberDataGroupArg,
            },
            shared_global_systems::sgs::Test as gs_Test,
            shared_global_systems::sgs::{
                AllLive, AlwaysLive, GSFlowDG, GSFlowTester, ManualLifetimeGS, TestBefore,
                WhenRequiredGS,
            },
            shared_local_systems::sls::{Test, TestAdder, TestAssertNumber4, TestMultiplier},
        },
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

        let global_allocator = EntityAllocator::get_global();
        let mut entity_ptr = global_allocator.write().allocate();
        entity_ptr.init(1, spawn_desc);

        let entity = entity_ptr.read();
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

        let global_allocator = EntityAllocator::get_global();
        let mut entity_ptr = global_allocator.write().allocate();
        entity_ptr.init(1, spawn_desc);

        let mut entity = entity_ptr.write();

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
        let new_world_id = es.create_world();

        es.step_world(0.0, 0.0, new_world_id); // Process reset

        for _ in 0..100 {
            let mut spawn_desc = EntitySpawnDescription::default();
            let init_params = Box::new(TestNumberDataGroupArg { num: 1 });

            TestNumberDataGroup::prepare_spawn(&mut spawn_desc, init_params);
            TestAdder::simple_prepare(&mut spawn_desc);
            TestMultiplier::simple_prepare(&mut spawn_desc);
            TestAssertNumber4::simple_prepare(&mut spawn_desc);
            spawn_desc.check_local_systems_panic();

            spawn_desc.set_name("Test Name".to_owned());

            es.create_entity(new_world_id, spawn_desc)
                .expect("Failed to create entity!");
        }

        // Create a weird entity to test the global system run
        let mut spawn_desc = EntitySpawnDescription::default();
        GSFlowDG::prepare_spawn(&mut spawn_desc);
        GSFlowTester::simple_prepare(&mut spawn_desc);
        spawn_desc.check_datagroups_panic(); // Should not panic
        spawn_desc.set_name("GSFlow Entity".to_owned());

        let entity_id = es
            .create_entity(new_world_id, spawn_desc)
            .expect("Failed to create entity!");

        es.step_world(0.0, 0.0, new_world_id);

        // Check that the entity with `GSFlowDG` has the right state
        let world = es.get_worlds().get(&new_world_id).unwrap();
        let entities = world.get_entities();
        let entity_lock = entities
            .get(&entity_id)
            .expect("Should retrieve entity that was previously created");
        let entity = entity_lock.read();
        let gs_flow_dg = entity
            .get_datagroup::<GSFlowDG>()
            .expect("This entity should be created with this datagroup");
        assert_eq!(
            gs_flow_dg.id, 1,
            "GSFlowTester Global system didn't run properly"
        );

        // Check that the state of the `GSFlowTester` is as expected
        let global_systems_lock = world.get_global_systems().read();
        let gs_storage_lock = global_systems_lock[get_id!(GSFlowTester) as usize]
            .as_ref()
            .expect("This global system should have storage loaded right")
            .read();

        let gs_storage: &GSFlowTester = cast(&*gs_storage_lock);
        assert_eq!(gs_storage.n_entities, 1);

        es.destroy_world(new_world_id);
    }

    #[test]
    fn test_parenting() {
        if !App::is_initialized() {
            App::initialize();
        }

        let es = EntitySystem::get();
        let new_world_id = es.create_world();
        es.step_world(0.0, 0.0, new_world_id); // Process world creation

        let get_spawn_desc = || {
            let mut desc = EntitySpawnDescription::default();
            Transform::prepare_spawn(&mut desc, Box::new(Transform::default()));
            AllLive::simple_prepare(&mut desc);
            desc
        };

        let root_id = es
            .create_entity(new_world_id, get_spawn_desc())
            .expect("Creation should be successful");

        let node_id = es
            .create_entity(new_world_id, get_spawn_desc())
            .expect("Creation should be successful");

        let mut spawn_desc = get_spawn_desc();
        Test::simple_prepare(&mut spawn_desc);
        AnimationDataGroup::prepare_spawn(
            &mut spawn_desc,
            Box::new(AnimationDataGroup {
                clip_name: "anim".into(),
                duration: 3.0,
            }),
        );
        let leaf_node_id = es
            .create_entity(new_world_id, spawn_desc)
            .expect("Creation should be successful");

        es.step_world(0.0, 0.0, new_world_id); // Process entity creation
        let root_ptr = es.get_entity(new_world_id, root_id);
        let node_ptr = es.get_entity(new_world_id, node_id);
        let leaf_node_ptr = es.get_entity(new_world_id, leaf_node_id);

        node_ptr.write().set_parent(root_ptr);
        leaf_node_ptr.write().set_parent(node_ptr);

        // Check that the nodes are parented as expected
        assert!(root_ptr.read().is_root());
        assert!(!node_ptr.read().is_root());

        {
            // Check that the counters make sense
            let root = root_ptr.read();
            let root_transform = root.get_transform().unwrap();
            assert_eq!(root_transform.n_nodes, 3);
            assert_eq!(root_transform.stage_count[0].load(Ordering::Acquire), 1);
            assert_eq!(root_transform.stage_count[1].load(Ordering::Acquire), 1);
            for i in 2..STAGE_COUNT {
                assert_eq!(root_transform.stage_count[i].load(Ordering::Acquire), 0);
            }

            let node = node_ptr.read();
            let node_transform = node.get_transform().unwrap();
            assert_eq!(node_transform.n_nodes, 2);

            let leaf_node = leaf_node_ptr.read();
            let leaf_node_transform = leaf_node.get_transform().unwrap();
            assert_eq!(leaf_node_transform.n_nodes, 1);
        }

        {
            // Check that parent is properly set
            let node = node_ptr.read();
            let node_transform = node.get_transform().unwrap();
            assert!(node_transform.parent.is_some());
            assert_eq!(node_transform.parent.unwrap(), root_ptr);
        }

        {
            // Check that deleting an intermediate node deletes the entire subtree
            es.destroy_entity(new_world_id, node_id);
            // force to delete
            es.step_world(0.0, 0.0, new_world_id);

            assert!(!node_ptr.is_live());
            assert!(!leaf_node_ptr.is_live());
            assert!(root_ptr.is_live());

            // Check that the root node is properly set
            let root = root_ptr.read();
            let root_transform = root.get_transform().unwrap();
            assert_eq!(root_transform.n_nodes, 1);
            assert!(root_transform.children.is_empty());
        }

        // Check that all entities passed to global systems are live
        es.step_world(0.0, 0.0, new_world_id);
        es.destroy_world(new_world_id);
    }

    #[test]
    fn test_hierarchy_update() {
        if !App::is_initialized() {
            App::initialize();
        }

        // Test hierarchical updates
        let es = EntitySystem::get();
        let new_world_id = es.create_world();
        es.step_world(0.0, 0.0, new_world_id); // Process world creation

        fn get_new_desc() -> EntitySpawnDescription {
            let mut desc = EntitySpawnDescription::default();
            Transform::prepare_spawn(&mut desc, Box::new(Transform::default()));
            desc
        }

        let mut root_desc = get_new_desc();
        root_desc.set_name("Root entity".into());
        let root_id = es
            .create_entity(new_world_id, root_desc)
            .expect("Creation should be successful");

        let mut desc = get_new_desc();
        desc.set_name("node entity".into());
        TestNumberDataGroup::prepare_spawn(&mut desc, Box::new(TestNumberDataGroupArg { num: 1 }));
        TestAdder::simple_prepare(&mut desc);
        TestMultiplier::simple_prepare(&mut desc);
        TestAssertNumber4::simple_prepare(&mut desc);

        let node_id = es
            .create_entity(new_world_id, desc)
            .expect("Node creation should be successful");
        {
            let worlds = es.get_world_map();
            let world = worlds.get(&new_world_id).unwrap();
            world.set_entity_parent(node_id, root_id);
        }
        es.step_world(0.0, 0.0, new_world_id); // Force entity creation

        {
            // Check that the root node is consistent
            let root_ptr = es.get_entity(new_world_id, root_id);
            let root = root_ptr.read();
            let root_transform = root.get_transform().unwrap();
            assert_eq!(
                root_transform.stage_count[0].load_value(),
                1,
                "Root node is not properly counting local systems for its children"
            );
        }

        let node_ptr = es.get_entity(new_world_id, node_id);
        assert!(node_ptr.is_live());

        {
            // Check that the node state finished properly
            let node = node_ptr.read();
            let number_dg = node
                .get_datagroup::<TestNumberDataGroup>()
                .expect("Number datagroup should be added to this node");

            assert_eq!(number_dg.num, 4);
        }

        // Check that all entities passed to the global systems are live after deletion
        es.destroy_world(new_world_id);
    }

    #[test]
    fn testing_global_system_lifetimes() {
        // Test that local systems are created and live as long as they should.
        let es = EntitySystem::get();
        let new_world_id = es.create_world();
        es.step_world(0.0, 0.0, new_world_id); // Process world creation

        // Global systems that are `AlwaysLive` should be active by now
        let worlds = es.get_worlds();
        let new_world = worlds.get(&new_world_id).unwrap();
        assert!(
            new_world.global_system_is_loaded::<AlwaysLive>(),
            "An AlwaysLive global system should be loaded by now"
        );

        // `WhenRequired` global system and `Manual` Global system shouldn't be live by now
        assert!(
            !new_world.global_system_is_loaded::<WhenRequiredGS>(),
            "WhenRequired global system should not be loaded by if not required"
        );
        assert!(
            !new_world.global_system_is_loaded::<ManualLifetimeGS>(),
            "A Manual Lifetime global system should not be loaded by if not requested"
        );

        // Request a GS creation
        new_world.load_global_system::<ManualLifetimeGS>();
        es.step_world(0.0, 0.0, new_world_id); // Process GS creation
        assert!(
            new_world.global_system_is_loaded::<ManualLifetimeGS>(),
            "ManualLifetimeGS should be loaded by now"
        );

        // Create an entity that requires the `WhenRequiredGS`
        let mut spawn = EntitySpawnDescription::new();
        WhenRequiredGS::simple_prepare(&mut spawn);
        let _ = es
            .create_entity(new_world_id, spawn)
            .expect("Should be able to create entity");
        es.step_world(0.0, 0.0, new_world_id); // Process Entity creation

        // Check that the WhenRequired global system is loaded by now
        assert!(
            new_world.global_system_is_loaded::<WhenRequiredGS>(),
            "WhenRequired Global system should be loaded when an entity requires it"
        );
    }

    #[test]
    #[should_panic]
    fn test_load_of_non_manual_fails() {
        // Test that local systems are created and live as long as they should.
        let es = EntitySystem::get();
        let new_world_id = es.create_world();
        es.step_world(0.0, 0.0, new_world_id); // Process world creation

        // Global systems that are `AlwaysLive` should be active by now
        let worlds = es.get_worlds();
        let new_world = worlds.get(&new_world_id).unwrap();

        new_world.load_global_system::<WhenRequiredGS>();
    }

    #[test]
    #[should_panic]
    fn test_entity_creation_with_missing_gs_should_panic() {
        // Test that local systems are created and live as long as they should.
        let es = EntitySystem::get();
        let new_world_id = es.create_world();
        es.step_world(0.0, 0.0, new_world_id); // Process world creation

        // Global systems that are `AlwaysLive` should be active by now
        let mut spawn = EntitySpawnDescription::new();
        ManualLifetimeGS::simple_prepare(&mut spawn);
        let _ = es.create_entity(new_world_id, spawn);

        // Should panic here
        es.step_world(0.0, 0.0, new_world_id);
    }

    #[test]
    #[should_panic]
    fn test_unload_when_required_fails_if_entity_exists() {
        let es = EntitySystem::get();
        let new_world_id = es.create_world();
        es.step_world(0.0, 0.0, new_world_id); // Process world creation

        let mut spawn = EntitySpawnDescription::new();
        WhenRequiredGS::simple_prepare(&mut spawn);
        let _ = es.create_entity(new_world_id, spawn);
        es.step_world(0.0, 0.0, new_world_id);

        let worlds = es.get_worlds();
        let world = worlds.get(&new_world_id).unwrap();

        world.unload_global_system::<WhenRequiredGS>();
        // Should panic here
        es.step_world(0.0, 0.0, new_world_id);
    }
}
