#[cfg(test)]
pub mod sdg {
    use proto_ecs::data_group::*;

    use crate::core::casting::CanCast;
    // -- first example datagroup
    #[derive(CanCast)]
    pub struct AnimationDataGroup {
        pub clip_name: String,
        pub duration: f64,
    }

    fn animation_factory() -> Box<dyn DataGroup> {
        return Box::new(AnimationDataGroup {
            clip_name: "Hello world".to_string(),
            duration: 12.4,
        });
    }

    register_datagroup_init!(AnimationDataGroup, Arg(AnimationDataGroup));

    impl AnimationDataGroupDesc for AnimationDataGroup {
        fn init(&mut self, init_data: Box<AnimationDataGroup>) {
            self.clip_name = init_data.clip_name.clone();
            self.duration = init_data.duration;
        }
    }

    register_datagroup!(AnimationDataGroup, animation_factory);

    // -- Second example datagroup

    #[derive(CanCast)]
    pub struct MeshDataGroup {}

    fn mesh_factory() -> Box<dyn DataGroup> {
        return Box::new(MeshDataGroup {});
    }

    register_datagroup_init!(MeshDataGroup, NoArg);

    impl MeshDataGroupDesc for MeshDataGroup {
        fn init(&mut self) {
            todo!()
        }
    }

    register_datagroup!(MeshDataGroup, mesh_factory);
}
