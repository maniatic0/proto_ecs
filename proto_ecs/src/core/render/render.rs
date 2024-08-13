
use lazy_static::lazy_static;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::render::render_api::{API, RenderCommand};
use proto_ecs::core::window::window_manager::WindowManager;

#[derive(Debug, Default)]
pub struct SceneDescription {

}

#[derive(Debug, Default)]
pub struct SceneData {

}

pub struct Render {
    scene_begun : bool,
    scene_data : SceneData
}

lazy_static!{
    static ref RENDER : RwLock<Option<Render>> = RwLock::default();
}

impl Render {
    pub fn init() {
        let mut render = RENDER.write();
        debug_assert!(render.is_none(), "Render already initialized");
        *render = Some(Render{scene_begun : false, scene_data: SceneData::default()});
        RenderCommand::initialize(WindowManager::get_platform());
    }

    pub fn begin_scene(scene_description : &SceneDescription) {
        let mut render_ref = RENDER.write();
        let render = render_ref.as_mut().expect("Render not yet initialized");

        render.scene_begun = true;
        // TODO
    }

    pub fn end_scene() {
        let mut render_ref = RENDER.write();
        let render = render_ref.as_mut().expect("Render not yet initialized");

        render.scene_begun = false;
        // TODO
    }

    pub fn on_window_resize(new_width : u32, new_height : u32) {
        // ? Not sure what the x,y parameters mean. They come from the OpenGL API
        RenderCommand::set_viewport(0, 0, new_width, new_height);
    }

    #[inline(always)]
    pub fn get_current_api() -> API {
        RenderCommand::get_current_api()
    }
}