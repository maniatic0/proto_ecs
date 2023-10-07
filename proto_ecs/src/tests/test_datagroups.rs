// -- < Testing datagroups API > ---------------------------
#[cfg(test)]
pub mod datagroup_test {
    use crate::{cast_mut, create_datagroup, get_id};
    use proto_ecs::data_group::*;

    use super::super::shared_datagroups::*;

    #[test]
    fn test_datagroup_registration() {
        if !DataGroupRegistry::get_global_registry()
            .read()
            .is_initialized()
        {
            let mut global_registry = DataGroupRegistry::get_global_registry().write();
            global_registry.init();
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
    }

    #[test]
    fn test_construction_workflow() {
        // Init registry just in case
        if !DataGroupRegistry::get_global_registry()
            .read()
            .is_initialized()
        {
            let mut global_registry = DataGroupRegistry::get_global_registry().write();
            global_registry.init();
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
        if !DataGroupRegistry::get_global_registry()
            .read()
            .is_initialized()
        {
            let mut global_registry = DataGroupRegistry::get_global_registry().write();
            global_registry.init();
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
    fn test_datagroup_initialization()
    {
        // TODO this setup should be done somewhere else, tests should not have to do this 
        // TODO Also it would be more convenient to have an `init_global_registry` function as a shortcut
        if !DataGroupRegistry::get_global_registry()
            .read()
            .is_initialized()
        {
            let mut global_registry = DataGroupRegistry::get_global_registry().write();
            global_registry.init();
        }

        let mut anim_datagroup = create_datagroup!(AnimationDataGroup);
        let init_params = AnimationDataGroup {
            clip_name: "hello world".to_string(),
            duration: 4.20,
        };
        anim_datagroup.init(Box::from(init_params));

        let anim_datagroup = cast_mut!(anim_datagroup, AnimationDataGroup);
        assert_eq!(anim_datagroup.clip_name.as_str(), "hello world");
        assert_eq!(anim_datagroup.duration, 4.20);
    }
}
