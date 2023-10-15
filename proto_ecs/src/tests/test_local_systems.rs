#[cfg(test)]
mod local_system_test {
    use super::super::shared_datagroups::sdg::*;
    use crate::{app::App, core::casting::cast, get_id, local_systems::LocalSystemRegistry};
    use proto_ecs::data_group::*;
    use proto_ecs::local_systems::register_local_system;

    // -- Local system creation
    struct Test;

    register_local_system! {
        Test,
        dependencies = (AnimationDataGroup, MeshDataGroup),
        stages = (0, 1)
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
        fn stage_0(_animation_data_group: &mut AnimationDataGroup, _mesh_data_group:Option<&mut MeshDataGroup>) {
            
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


