// -- < Testing datagroups API > ---------------------------
#[cfg(test)]
pub mod datagroup_test {
    use crate::{
        app::App,
        core::casting::{cast, cast_mut},
        create_datagroup,
        entities::entity_spawn_desc::EntitySpawnDescription,
        get_id,
    };
    use proto_ecs::data_group::*;
    use crate::core::common::InitDesc;

    use super::super::shared_datagroups::sdg::*;

    #[test]
    fn test_datagroup_registration() {
        if !App::is_initialized() {
            App::initialize();
        }

        let global_registry = DataGroupRegistry::get_global_registry().read();

        let anim_id = get_id!(AnimationDataGroup);
        let mesh_id = get_id!(MeshDataGroup);

        let anim_entry = global_registry.get_entry_by_id(anim_id);
        let mesh_entry = global_registry.get_entry_by_id(mesh_id);
        assert_eq!(anim_entry.id, anim_id);
        assert_eq!(mesh_entry.id, mesh_id);
        assert_eq!(
            global_registry.get_entry::<AnimationDataGroup>().id,
            anim_id
        );
        assert_eq!(global_registry.get_entry::<MeshDataGroup>().id, mesh_id);

        assert_eq!(anim_entry.name, AnimationDataGroup::NAME);
        assert_eq!(mesh_entry.name, MeshDataGroup::NAME);

        assert_eq!(anim_entry.name_crc, AnimationDataGroup::NAME_CRC);
        assert_eq!(mesh_entry.name_crc, MeshDataGroup::NAME_CRC);

        assert_eq!(anim_entry.factory_func, AnimationDataGroup::FACTORY);
        assert_eq!(mesh_entry.factory_func, MeshDataGroup::FACTORY);
    }

    #[test]
    fn test_construction_workflow() {
        if !App::is_initialized() {
            App::initialize();
        }

        let anim_datagroup = create_datagroup!(AnimationDataGroup);
        let mesh_datagroup = create_datagroup!(MeshDataGroup);

        let mesh_id = get_id!(MeshDataGroup);
        let anim_id = get_id!(AnimationDataGroup);

        assert_eq!(
            mesh_datagroup.get_id(),
            mesh_id,
            "Mesh id from object is not the same as mesh id from class"
        );
        assert_eq!(
            anim_datagroup.get_id(),
            anim_id,
            "Anim id from object is not the same as anim id from class"
        );
        assert_ne!(mesh_datagroup.get_id(), anim_datagroup.get_id());
    }

    #[test]
    fn test_init_registry() {
        if !App::is_initialized() {
            App::initialize();
        }

        let global_registry = DataGroupRegistry::get_global_registry().read();
        for (i, item) in global_registry.into_iter().enumerate() {
            assert_eq!(
                i as u32, item.id,
                "Items should be sorted after init so that item accessing is just array indexing"
            );
        }
    }

    #[test]
    fn test_datagroup_initialization() {
        if !App::is_initialized() {
            App::initialize();
        }

        assert_eq!(AnimationDataGroup::INIT_DESC, InitDesc::Arg);
        assert_eq!(
            <AnimationDataGroup as DataGroupInitDescTrait>::ArgType::INIT_DESC,
            InitDesc::Arg
        );
        assert_eq!(
            proto_ecs::data_group::DataGroupRegistry::get_global_registry()
                .read()
                .get_entry::<AnimationDataGroup>()
                .init_desc,
            InitDesc::Arg
        );

        let mut anim_datagroup = create_datagroup!(AnimationDataGroup);
        let init_params = AnimationDataGroup {
            clip_name: "hello world".to_string(),
            duration: 4.20,
        };
        anim_datagroup.__init__(Some(Box::from(init_params)));

        let anim_datagroup: &mut AnimationDataGroup = cast_mut(&mut anim_datagroup);
        assert_eq!(anim_datagroup.clip_name.as_str(), "hello world");
        assert_eq!(anim_datagroup.duration, 4.20);
    }

    #[test]
    fn test_datagroup_entity_spawn_desc() {
        if !App::is_initialized() {
            App::initialize();
        }

        let mut spawn_desc = EntitySpawnDescription::default();
        let init_params = Box::new(AnimationDataGroup {
            clip_name: "hello world".to_string(),
            duration: 4.20,
        });

        let empty = AnimationDataGroup::prepare_spawn(&mut spawn_desc, init_params);

        spawn_desc.check_datagroups_panic();
        assert!(empty.is_none());

        let init_params = spawn_desc.get_datagroup::<AnimationDataGroup>();
        assert!(init_params.is_some());
        let init_params = init_params.expect("Failed to add test params!");

        let init_params = match init_params {
            DataGroupInitType::Uninitialized(_) => panic!("Unexpected init arg type!"),
            DataGroupInitType::NoInit => panic!("Unexpected init arg type!"),
            DataGroupInitType::NoArg => panic!("Unexpected init arg type!"),
            DataGroupInitType::Arg(params) => params,
            DataGroupInitType::OptionalArg(_) => panic!("Unexpected init arg type!"),
        };

        let init_params: &AnimationDataGroup = cast(init_params);
        assert_eq!(init_params.clip_name.as_str(), "hello world");
        assert_eq!(init_params.duration, 4.20);
    }

    #[test]
    #[should_panic]
    fn test_datagroup_wrong_init_data() {
        if !App::is_initialized() {
            App::initialize();
        }

        let mut spawn_desc = EntitySpawnDescription::default();

        spawn_desc.add_datagroup::<AnimationDataGroup>(DataGroupInitType::NoInit);
        spawn_desc.check_datagroups_panic();
    }

    #[test]
    #[should_panic]
    fn test_datagroup_uninitialized_init_data() {
        if !App::is_initialized() {
            App::initialize();
        }

        let mut spawn_desc = EntitySpawnDescription::default();

        spawn_desc
            .add_datagroup::<AnimationDataGroup>(DataGroupInitType::Uninitialized("Test message"));
        spawn_desc.check_datagroups_panic();
    }
}
