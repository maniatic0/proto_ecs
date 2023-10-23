#[cfg(test)]
mod test {
    use crate::{
        app::App,
        core::ids::{HasID, IDLocator},
        entities::{
            entity::Entity,
            entity_spawn_desc::EntitySpawnDescription,
            entity_system::{EntitySystem, World, DEFAULT_WORLD},
        },
        tests::{
            shared_datagroups::sdg::{
                AnimationDataGroup, MeshDataGroup, TestNumberDataGroup, TestNumberDataGroupArg,
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
        es.step(0.0); // Process reset

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

        es.step(0.0);
    }
}
