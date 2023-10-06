
#[cfg(test)]
mod local_system_test
{
    use proto_ecs::data_group::*;
    use proto_ecs::get_id;

    use crate::{cast, cast_mut, core::casting::CanCast};

    // -- first example datagroup
    #[derive(CanCast)]
    pub struct AnimationDataGroup
    {
        pub clip_name : String,
        pub duration : f64
    }

    impl DataGroup for AnimationDataGroup
    {
        fn init(&mut self, _init_data : Box<dyn CanCast>) 
        {
            let init_data = cast!(_init_data, AnimationDataGroup);
            self.clip_name = init_data.clip_name.clone();
            self.duration = init_data.duration;
        }
    }

    fn animation_factory() -> Box<dyn DataGroup>
    {
        return Box::new(AnimationDataGroup{
            clip_name : "Hello world".to_string(),
            duration : 12.4
        });
    }

    register_datagroup!(AnimationDataGroup, animation_factory);

    // -- Second example datagroup

    #[derive(CanCast)]
    pub struct MeshDataGroup
    { }

    impl DataGroup for MeshDataGroup
    {
        fn init(&mut self, _init_data : Box<dyn CanCast>) 
        {
        }
    }

    fn mesh_factory() -> Box<dyn DataGroup>
    {
        return Box::new(MeshDataGroup{});
    }

    register_datagroup!(MeshDataGroup, mesh_factory);

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