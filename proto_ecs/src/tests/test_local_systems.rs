
#[cfg(test)]
mod local_system_test
{
    use proto_ecs::data_group::*;
    use proto_ecs::local_systems::local_system;
    use crate::{local_systems::LocalSystemRegistry, get_id, core::casting::cast, app::App};
    use super::super::shared_datagroups::sdg::*;

    // -- Local system creation

    // This function is provided by an user
    #[local_system]
    fn my_local_system(_anim : &mut AnimationDataGroup, _mesh : &mut MeshDataGroup)
    {
        // do something here
        _anim.duration = 4.20;
    }

    #[test]
    fn test_local_system_registration()
    {
        if !App::is_initialized()
        {
            App::initialize();
        }

        let dg_registry = DataGroupRegistry::get_global_registry().read();
        let ls_registry = LocalSystemRegistry::get_global_registry().read();
        let mesh = dg_registry.create::<MeshDataGroup>();
        let anim = dg_registry.create::<AnimationDataGroup>();
        let mut dgs = vec![anim, mesh];
        let indices : [usize; 2] = [0,1];
        // TODO we need a better way to locate a system
        let id = 3066720040u32; // Hardcoded crc32 from the local system function
        let entry = ls_registry.get_entry_by_id(id);
        (entry.func)(&indices, &mut dgs);

        let anim: &AnimationDataGroup = cast(&dgs[0]);
        assert_eq!(anim.duration, 4.20, "System is not affecting the intended datagroup");
        assert_eq!(entry.dependencies.len(), 2, "There should be two dependencies for this system");
        assert!(
            entry.dependencies[0] == get_id!(AnimationDataGroup) && entry.dependencies[1] == get_id!(MeshDataGroup), 
            "Inconsistent dependencies for local system"
        )
    }

    // This is an example of the function generated by the local_system macro
    // fn __my_local_system__(indices : &[usize], entity_datagroups : &mut Vec<Box<dyn DataGroup>>)
    // {
    //     debug_assert!({
    //         let mut unique_set = std::collections::HashSet::new();
    //         indices.iter().all(|&i| {unique_set.insert(i) && i < entity_datagroups.len()})
    //     }, "Overlapping indices or index out of range");

    //     unsafe 
    //     {
    //         let entity_datagroups_ptr = entity_datagroups.as_mut_ptr();

    //         let anim = &mut *entity_datagroups_ptr.add(indices[0]);
    //         let mesh = &mut *entity_datagroups_ptr.add(indices[1]);

    //         let anim = cast_mut!(anim, AnimationDataGroup);
    //         let mesh = cast_mut!(mesh, MeshDataGroup);
    //         my_local_system(anim, mesh);
    //     }

    // }
}


