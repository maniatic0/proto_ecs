use std::{
    collections::HashMap,
    mem,
    sync::atomic::{AtomicBool, Ordering},
};

use lazy_static::lazy_static;
use parking_lot::RwLock;

use crate::{
    core::assets_management::models::ModelHandle, entities::transform_datagroup::TransformMatrix,
};

use super::{
    buffer::{BufferElement, BufferLayout},
    camera::Camera,
    material::{Material, MaterialHandle},
    render_api::{IndexBufferHandle, RenderCommand, VertexArrayHandle, VertexBufferHandle},
    shader::{DataType, Precision, ShaderDataType, ShaderDataTypeValue},
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

            // Update the data used to draw the current frame
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
    fn is_last_frame_finished() -> bool {
        RENDER_THREAD_SHARED_STORAGE
            .last_frame_finished
            .load(Ordering::SeqCst)
    }

    fn render(&mut self) {
        RenderCommand::set_clear_color(glam::vec4(1.0, 0.5, 0.5, 1.0));
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
            if let Some(_) = self.models_in_gpu.get(&proxy.model) {
                continue;
            }

            models_to_load.push(proxy.model);
        }

        for model in models_to_load {
            self.load_model(model);
        }
    }

    fn render_frame(&mut self) {
        let render_lock = Render::get();
        let render = render_lock.read();
        let render = render.as_ref().unwrap();

        for proxy in self.current_frame_desc.render_proxies.iter() {
            let material = render.materials.get(proxy.material) as &Material;
            let gpu_model_data = self
                .models_in_gpu
                .get(&proxy.model)
                .expect("This model should be in gpu by now");

            RenderCommand::bind_vertex_array(gpu_model_data.vertex_array);
            RenderCommand::bind_shader(material.shader);

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

                RenderCommand::draw_indexed(gpu_model_data.vertex_array);
            }
        }
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
        let vao = RenderCommand::create_vertex_array();
        let vbo = RenderCommand::create_vertex_buffer(model.vertices());
        RenderCommand::set_vertex_buffer_layout(
            vbo,
            BufferLayout::from_elements(vec![BufferElement::new(
                "a_Position".into(),
                ShaderDataType::new(Precision::P32, DataType::Float3),
                false,
            )]),
        );
        let ibo = RenderCommand::create_index_buffer(model.indices());

        RenderCommand::set_vertex_array_index_buffer(vao, ibo);
        RenderCommand::set_vertex_array_vertex_buffer(vao, vbo);

        self.models_in_gpu.insert(
            model_handle,
            ModelData {
                vertex_buffer: vbo,
                index_buffer: ibo,
                vertex_array: vao,
            },
        );
    }

    fn frame_finished() {
        RENDER_THREAD_SHARED_STORAGE
            .last_frame_finished
            .store(true, Ordering::SeqCst);
    }
}
