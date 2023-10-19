#[cfg(test)]
mod test {
    use crate::{
        app::App,
        entities::{entity::Entity, entity_spawn_desc::EntitySpawnDescription},
        tests::{
            shared_datagroups::sdg::{AnimationDataGroup, MeshDataGroup},
            shared_local_systems::sls::Test,
        }, core::ids::IDLocator,
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
            Some(dg) if dg.get_id() == AnimationDataGroup::get_id()
        ));
        assert!(matches!(
            entity.get_datagroup::<MeshDataGroup>(),
            Some(dg) if dg.get_id() == MeshDataGroup::get_id()
        ));
        assert!(entity.contains_local_system::<Test>());
    }
}
