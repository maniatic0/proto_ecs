
// -- < Testing datagroups API > ---------------------------
#[cfg(test)]
mod datagroup_test
{
    use proto_ecs::data_group::*;

    use crate::{get_id, create_datagroup, cast, cast_mut, core::casting::CanCast};

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

    #[test]
    fn test_datagroup_registration()
    {
        let global_registry = DataGroupRegistry::get_global_registry();
        let anim_id  = get_id!(AnimationDataGroup);
        let mesh_id  = get_id!(MeshDataGroup);

        let registry = global_registry.lock().unwrap();

        let anim_entry = registry.get_entry_of(anim_id);
        let mesh_entry = registry.get_entry_of(mesh_id);

        assert_eq!(anim_entry.id, anim_id);
        assert_eq!(mesh_entry.id, mesh_id);
    }

    #[test]
    fn test_construction_workflow()
    {
        let anim_datagroup = create_datagroup!(AnimationDataGroup);
        let mesh_datagroup = create_datagroup!(MeshDataGroup);

        let mesh_id = get_id!(MeshDataGroup);
        let anim_id = get_id!(AnimationDataGroup);

        assert_eq!(mesh_datagroup.get_id(), mesh_id, "Mesh id from object is not the same as mesh id from class");
        assert_eq!(anim_datagroup.get_id(), anim_id, "Anim id from object is not the same as anim id from class");
        assert_ne!(mesh_datagroup.get_id(), anim_datagroup.get_id());
    }

    #[test]
    fn test_init_registry()
    {
        if let Ok(registry) = DataGroupRegistry::get_global_registry().lock().as_mut()
        {
            registry.init();
            for (i, item) in registry.into_iter().enumerate()
            {
                assert_eq!(i as u32, item.id, "Items should be sorted after init so that item accessing is just array indexing");
            }
        }
        else {
            assert!(false, "Could not get lock to global registry because it was poisoned")
        }
    }

    #[test]
    fn test_datagroup_initialization()
    {
        let mut anim_datagroup = create_datagroup!(AnimationDataGroup);
        let init_params = AnimationDataGroup{clip_name:"hello world".to_string(), duration: 4.20};
        anim_datagroup.init(Box::from(init_params));
        
        let anim_datagroup = cast_mut!(anim_datagroup, AnimationDataGroup);
        assert_eq!(anim_datagroup.clip_name.as_str(), "hello world");
        assert_eq!(anim_datagroup.duration, 4.20);
    }
}