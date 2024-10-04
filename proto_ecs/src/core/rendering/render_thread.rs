use std::{
    collections::HashMap,
    mem,
    sync::atomic::{AtomicBool, Ordering},
    thread::sleep,
    time::Duration,
};

use lazy_static::lazy_static;
use parking_lot::RwLock;
use proto_ecs::core::rendering::shader::ShaderError;
use scc::Queue;

use crate::{
    core::{assets_management::models::ModelHandle, windowing::window_manager::WindowManager},
    entities::transform_datagroup::TransformMatrix,
};

use super::{
    buffer::{BufferElement, BufferLayout},
    camera::Camera,
    material::{Material, MaterialHandle},
    render_api::{
        IndexBufferHandle, RenderCommand, ShaderHandle, VertexArrayHandle, VertexBufferHandle,
    },
    shader::{DataType, Precision, ShaderDataType, ShaderDataTypeValue, ShaderSrc},
    Render,
};

pub struct RenderThread {
    current_frame_desc: FrameDesc,
    models_in_gpu: HashMap<ModelHandle, ModelData>,
}

/// Storage shared between the render thread and the main thread.
///
/// Note that this is a separate object of the internal render storage.
/// This is helpful to prevent data accessed only from the render thread
/// to require a lock to be accessed
#[derive(Default)]
pub struct RenderSharedStorage {
    last_frame_finished: AtomicBool,
    running: AtomicBool,
    started: AtomicBool,

    /// Description of the next frame.
    frame_desc: RwLock<FrameDesc>,

    /// Store shaders by name, for easier retrieval
    name_to_shaders: RwLock<HashMap<String, ShaderHandle>>,
}

struct ModelData {
    vertex_buffer: VertexBufferHandle,
    index_buffer: IndexBufferHandle,
    vertex_array: VertexArrayHandle,
}

/// A description of a frame to render.
///
/// Holds the data required to render a scene, like all the
/// render proxies, the camera, light descriptions and so on
#[derive(Debug, Default)]
pub struct FrameDesc {
    pub render_proxies: Vec<RenderProxy>,
    pub camera: Camera, // Lights not yet implemented
}

/// Render proxies should be POD only, so that they can be easily be copied
/// and sent between threads
#[derive(Debug, Clone, Copy)]
pub struct RenderProxy {
    pub model: ModelHandle,
    pub material: MaterialHandle,
    pub transform: TransformMatrix,
    pub position: macaw::Vec3,
}

lazy_static! {
    static ref RENDER_THREAD_STATE: RwLock<Option<RenderThread>> = RwLock::new(None);
}

lazy_static! {
    static ref RENDER_THREAD_SHARED_STORAGE: RenderSharedStorage = RenderSharedStorage::default();
}

impl RenderThread {
    fn init(&mut self) {
        RENDER_THREAD_SHARED_STORAGE
            .last_frame_finished
            .store(true, Ordering::SeqCst);
        RENDER_THREAD_SHARED_STORAGE
            .running
            .store(false, Ordering::SeqCst);
        RENDER_THREAD_SHARED_STORAGE
            .started
            .store(false, Ordering::SeqCst);

        self.load_default_shaders();
    }

    pub fn new() -> Self {
        RenderThread {
            current_frame_desc: FrameDesc::default(),
            models_in_gpu: HashMap::new(),
        }
    }

    pub fn start(&mut self) {
        self.init();
        self.run();
    }

    #[inline(always)]
    pub fn get_next_frame_desc() -> &'static RwLock<FrameDesc> {
        &RENDER_THREAD_SHARED_STORAGE.frame_desc
    }

    pub fn run(&mut self) {
        RENDER_THREAD_SHARED_STORAGE
            .running
            .store(true, Ordering::SeqCst);
        RENDER_THREAD_SHARED_STORAGE
            .started
            .store(true, Ordering::SeqCst);

        while RenderThread::is_running() {
            // TODO Change for a cond variable or something better
            // Busywaiting until last frame is outdated
            if RenderThread::is_last_frame_finished() {
                continue;
            }

            // Update the data used to draw current frame
            self.update_current_frame_desc();

            // Actual rendering
            self.render();
        }
    }

    /// Mark the next frame data as updated.
    ///
    /// The render thread will not draw anything if the render thread is not
    /// updated
    pub fn next_frame_updated() {
        RENDER_THREAD_SHARED_STORAGE
            .last_frame_finished
            .store(false, Ordering::SeqCst);
    }

    /// Stop the render thread
    pub fn stop() {
        RENDER_THREAD_SHARED_STORAGE
            .running
            .store(false, Ordering::SeqCst);
    }

    pub fn is_started() -> bool {
        RENDER_THREAD_SHARED_STORAGE.started.load(Ordering::SeqCst)
    }

    fn update_current_frame_desc(&mut self) {
        let mut next_frame = RENDER_THREAD_SHARED_STORAGE.frame_desc.write();
        mem::swap(&mut self.current_frame_desc, &mut *next_frame);
    }

    #[inline(always)]
    fn is_running() -> bool {
        RENDER_THREAD_SHARED_STORAGE.running.load(Ordering::SeqCst)
    }

    #[inline(always)]
    pub fn is_last_frame_finished() -> bool {
        RENDER_THREAD_SHARED_STORAGE
            .last_frame_finished
            .load(Ordering::SeqCst)
    }

    fn render(&mut self) {
        RenderCommand::clear();
        {
            self.send_models_to_gpu();
            self.render_frame();
        }
        RenderThread::frame_finished();
    }

    /// Take all the models within the frame description
    /// to the gpu if they are not already in
    fn send_models_to_gpu(&mut self) {
        let mut models_to_load = vec![];
        for proxy in self.current_frame_desc.render_proxies.iter() {
            if let None = self.models_in_gpu.get(&proxy.model) {
                models_to_load.push(proxy.model);
            }
        }

        for model in models_to_load {
            self.load_model(model);
        }
    }

    fn render_frame(&mut self) {
        let render_lock = Render::get();
        let render = render_lock.read();
        let render = render.as_ref().unwrap();
        let mvp_camera = {
            let to_camera_matrix = self.current_frame_desc.camera.world_to_camera_matrix();

            let fov = f32::to_radians(80.0);
            let window_manager = WindowManager::get().read();
            let window = window_manager.get_window();
            let h = window.get_heigth() as f32;
            let w = window.get_width() as f32;
            let aspect_ratio = w as f32 / h as f32;

            self.current_frame_desc
                .camera
                .perspective_matrix(fov, aspect_ratio, 0.1, 100.0)
                * to_camera_matrix
        };

        for proxy in self.current_frame_desc.render_proxies.iter() {
            let material = render.materials.get(proxy.material) as &Material;
            let gpu_model_data = self
                .models_in_gpu
                .get(&proxy.model)
                .expect("This model should be in gpu by now");

            RenderCommand::bind_shader(material.shader);
            // Update current camera matrix:
            RenderCommand::set_shader_uniform_fmat4(material.shader, "u_Perspective", &mvp_camera);

            // DEBUG: Set eye position
            RenderCommand::set_shader_uniform_fvec3(
                material.shader,
                "u_EyePosition",
                &self.current_frame_desc.camera.get_position().into(),
            );

            // Set up transform
            let mut transform = macaw::Mat4::from_mat3(proxy.transform.matrix3.into());
            transform.w_axis =
                macaw::vec4(proxy.position.x, proxy.position.y, proxy.position.z, 1.0);
            RenderCommand::set_shader_uniform_fmat4(material.shader, "u_Transform", &transform);

            for (name, value) in material.parameters.iter() {
                match value {
                    ShaderDataTypeValue::Float_32(v) => {
                        RenderCommand::set_shader_uniform_f32(material.shader, name.as_str(), *v)
                    }
                    ShaderDataTypeValue::Float2_32(v) => {
                        RenderCommand::set_shader_uniform_fvec2(material.shader, name.as_str(), v)
                    }
                    ShaderDataTypeValue::Float3_32(v) => {
                        RenderCommand::set_shader_uniform_fvec3(material.shader, name.as_str(), v)
                    }
                    ShaderDataTypeValue::Float4_32(v) => {
                        RenderCommand::set_shader_uniform_fvec4(material.shader, name.as_str(), v)
                    }
                    ShaderDataTypeValue::Int_32(v) => {
                        RenderCommand::set_shader_uniform_i32(material.shader, name.as_str(), *v)
                    }
                    ShaderDataTypeValue::Mat3_32(v) => {
                        RenderCommand::set_shader_uniform_fmat3(material.shader, name.as_str(), v)
                    }
                    ShaderDataTypeValue::Mat4_32(v) => {
                        RenderCommand::set_shader_uniform_fmat4(material.shader, name.as_str(), v)
                    }

                    _ => unimplemented!("Data type not yet implemented"),
                }
            }
            RenderCommand::draw_indexed(gpu_model_data.vertex_array);
        }
        RenderCommand::finish();
    }
    fn load_model(&mut self, model_handle: ModelHandle) {
        debug_assert!(
            !self.models_in_gpu.contains_key(&model_handle),
            "This model is already in GPU"
        );
        let render_lock = Render::get();
        let render = render_lock.read();
        let render = render.as_ref().unwrap();

        let model = render.models.get(model_handle);
        let vertices = model.data();
        let vbo = RenderCommand::create_vertex_buffer(vertices.as_slice());
        RenderCommand::set_vertex_buffer_layout(
            vbo,
            BufferLayout::from_elements(vec![
                BufferElement::new(
                    "a_Position".into(),
                    ShaderDataType::new(Precision::P32, DataType::Float3),
                    false,
                ),
                BufferElement::new(
                    "a_Normal".into(),
                    ShaderDataType::new(Precision::P32, DataType::Float3),
                    true,
                ),
                BufferElement::new(
                    "a_UV".into(),
                    ShaderDataType::new(Precision::P32, DataType::Float2),
                    true,
                ),
            ]),
        );
        let indices = model.indices();
        let ibo = RenderCommand::create_index_buffer(indices);

        let vao = RenderCommand::create_vertex_array();
        RenderCommand::set_vertex_array_vertex_buffer(vao, vbo);
        RenderCommand::set_vertex_array_index_buffer(vao, ibo);

        self.models_in_gpu.insert(
            model_handle,
            ModelData {
                vertex_buffer: vbo,
                index_buffer: ibo,
                vertex_array: vao,
            },
        );
    }

    #[inline(always)]
    pub(crate) fn frame_finished() {
        RENDER_THREAD_SHARED_STORAGE
            .last_frame_finished
            .store(true, Ordering::SeqCst);
    }

    /// Load default materials used for debugging and displaying models
    fn load_default_shaders(&mut self) {
        const VERTEX_SRC: &str = "
                    #version 330 core
                    layout(location=0) in vec3 position;
                    layout(location=1) in vec3 normal;
                    layout(location=2) in vec2 vert_uvs;

                    uniform mat4 u_Transform; 
                    uniform mat4 u_Perspective; 
                    uniform vec3 u_EyePosition; 
                    
                    out vec4 vertex_position;
                    out vec3 transformed_normal;
                    out vec2 uvs; 
                    out vec3 eye_position_transformed;
                    out vec3 original_normal;

                    void main() {
                        mat4 modelview =  u_Perspective * u_Transform;
                        transformed_normal = mat3(transpose(inverse(u_Transform))) * normal;
                        gl_Position = modelview * vec4(position, 1.0);
                        vertex_position = u_Transform * vec4(position, 1.);
                        uvs = vert_uvs;
                        original_normal = normal;
                    }
                    \0";
        const FRAGMENT_SRC: &str = "
                    #version 330 core

                    uniform mat4 u_Transform; 
                    uniform mat4 u_Perspective; 
                    uniform vec3 u_EyePosition; 

                    out vec4 fragcolor;
                    in vec4 vertex_position;
                    in vec3 transformed_normal;
                    in vec2 uvs;
                    in vec3 original_normal;

                    vec3 phong(vec3 eye_position) {
                    
                        vec3 normal = normalize(transformed_normal);
                        vec3 frag_position = vertex_position.xyz / vertex_position.w;

                        // Surface properties
                        vec3 diffuse_color = vec3(.8, .8, .8);
                        vec3 specular_color = vec3(1);
                        vec3 ambient_color = vec3(.6, 0., .6);
                        float shininess = .2;

                        // Light properties
                        vec3 light_direction = -normalize(vec3(0., -1., 1.)); // From surface to light
                        vec3 light_color = vec3(.8);
                        float light_intensity = .7;
                        float ambient_intensity = .2;

                        // compute phong shading
                        vec3 final_color = vec3(0.);

                        // Ambient
                        final_color += ambient_color * ambient_intensity;

                        // Diffuse
                        float d = max(0, dot(normal, light_direction));
                        final_color += diffuse_color * light_intensity * d;

                        // Specular
                        vec3 to_eye = normalize(eye_position - frag_position);
                        vec3 half_vec = normalize(light_direction + to_eye);

                        final_color += specular_color * light_intensity * pow(max(0., dot(normal, half_vec)), shininess);

                        return final_color;
                    }

                    void main() {
                        fragcolor = vec4(phong(u_EyePosition), 1.);
                    }
                \0";
        let default_shader = match RenderCommand::create_shader(
            "default",
            ShaderSrc::Code(VERTEX_SRC),
            ShaderSrc::Code(FRAGMENT_SRC),
        ) {
            Result::Err(ShaderError::CompilationError(e)) => {
                panic!("Shader compilation error: \n{}", e)
            }
            Ok(s) => s,
            e => e.expect("Unable to create default shader"),
        };

        let mut name_to_shader = RENDER_THREAD_SHARED_STORAGE.name_to_shaders.write();
        name_to_shader.insert("default".into(), default_shader);

        RenderCommand::add_shader_uniform(
            default_shader,
            "u_Transform",
            ShaderDataType::new(Precision::P32, DataType::Mat4),
        )
        .expect("Should be able to add transform uniform to deafult shader");

        RenderCommand::add_shader_uniform(
            default_shader,
            "u_Perspective",
            ShaderDataType::new(Precision::P32, DataType::Mat4),
        )
        .expect("Should be able to add transform uniform to deafult shader");

        RenderCommand::add_shader_uniform(
            default_shader,
            "u_EyePosition",
            ShaderDataType::new(Precision::P32, DataType::Float3),
        )
        .expect("Could not add eye position uniform to shader")
    }

    pub fn get_shader_handle_from_name(name: &str) -> Option<ShaderHandle> {
        let name_to_shader = RENDER_THREAD_SHARED_STORAGE.name_to_shaders.read();
        let result = name_to_shader.get(name);

        result.map(|handle| handle.clone())
    }
}
