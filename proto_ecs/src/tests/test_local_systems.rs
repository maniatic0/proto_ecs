
#[cfg(test)]
mod local_system_test
{
    use proto_ecs::data_group::*;
    use proto_ecs::get_id;

    use crate::cast_mut;

    use super::super::shared_datagroups::*;

    // -- Local system creation

    // This function is provided by an user
    fn my_local_system(_anim : &mut AnimationDataGroup, _mesh : &mut MeshDataGroup)
    {
        // do something here
    }

    // This function should be implemented by a macro reading the above function
    fn __my_local_system__(indices : &[usize], entity_datagroups : &mut Vec<Box<dyn DataGroup>>)
    {
        let mut it = entity_datagroups.iter_mut();
        let anim= it.nth(indices[0]).unwrap();
        let mesh = it.nth(indices[1]).unwrap();
        

        let anim = cast_mut!(anim, AnimationDataGroup);
        let mesh = cast_mut!(mesh, MeshDataGroup);
        my_local_system(anim, mesh);
    }
}