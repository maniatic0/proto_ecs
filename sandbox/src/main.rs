use std::mem::size_of;

use proto_ecs::core::rendering::buffer::{BufferElement, BufferLayout};
use proto_ecs::core::rendering::render_api::{RenderCommand, ShaderHandle, VertexArrayHandle};
use proto_ecs::core::rendering::shader::{DataType, Precision, ShaderDataType, ShaderSrc};
use proto_ecs::core::windowing::events::Event;
use proto_ecs::core::windowing::window_manager::WindowManager;
use proto_ecs::prelude::*;

struct MyLayer {
    triangle_shader: Option<ShaderHandle>,
    triangle_data: Option<VertexArrayHandle>,
    color: macaw::Vec3,
}

// TODO Look for something to do in these cases
unsafe impl Send for MyLayer {}
unsafe impl Sync for MyLayer {}

const VERTEX_SRC: &str = "
#version 100
precision mediump float;
uniform vec3 u_Color;

attribute vec2 position;
attribute vec3 color;

varying vec3 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_color = color;
}
\0";

const FRAGMENT_SRC: &str = "
#version 100
precision mediump float;
uniform vec3 u_Color;
varying vec3 v_color;

void main() {
    gl_FragColor = vec4(u_Color, 1.0);
}
\0";

#[repr(C)]
struct VertexData {
    position: macaw::Vec2,
    color: macaw::Vec3,
}

// TODO We need a better way to cast custom data types to f32 arrays to send data to the GPU
unsafe fn any_as_f32_slice<T: Sized>(p: &T) -> &[f32] {
    ::core::slice::from_raw_parts(
        (p as *const T) as *const f32,
        ::core::mem::size_of::<T>() / size_of::<f32>(),
    )
}
impl Layer for MyLayer {
    fn on_attach(&mut self) {
        self.triangle_shader = Some({
            let shader = RenderCommand::create_shader("Example Triangle", ShaderSrc::Code(VERTEX_SRC), ShaderSrc::Code(FRAGMENT_SRC))
                .expect("Could not create triangle shader");
            RenderCommand::add_shader_uniform(shader, "u_Color", ShaderDataType{precision: Precision::P32, data_type: DataType::Float3})
                .expect("Should be able to add this uniform");
            shader
        });

        static VERTEX_DATA: [VertexData; 3] = [
            VertexData {
                position: macaw::vec2(-0.5, -0.5),
                color: macaw::vec3(1.0, 0.0, 0.0),
            },
            VertexData {
                position: macaw::vec2(0.0, 0.5),
                color: macaw::vec3(0.0, 1.0, 0.0),
            },
            VertexData {
                position: macaw::vec2(0.5, -0.5),
                color: macaw::vec3(0.0, 0.0, 1.0),
            },
        ];

        // Create a buffer for this triangle data
        let vbo = RenderCommand::create_vertex_buffer(unsafe { any_as_f32_slice(&VERTEX_DATA) });
        RenderCommand::set_vertex_buffer_layout(
            vbo,
            BufferLayout::from_elements(vec![
                BufferElement::new("a_Position".into(), ShaderDataType::new(Precision::P32, DataType::Float2), false),
                BufferElement::new("a_Color".into(), ShaderDataType::new(Precision::P32, DataType::Float3), false),
            ]),
        );
        let index_buffer = RenderCommand::create_index_buffer(&[0, 1, 2]);
        let vao = RenderCommand::create_vertex_array();
        RenderCommand::set_vertex_array_vertex_buffer(vao, vbo);
        RenderCommand::set_vertex_array_index_buffer(vao, index_buffer);

        self.triangle_data = Some(vao);
        println!("Triangle data intialized!");
    }

    fn on_detach(&mut self) {
        // TODO cleanup
    }

    fn update(&mut self, _delta_time: f32) {
        RenderCommand::set_clear_color(macaw::vec4(1.0, 0.5, 0.5, 1.0));
        RenderCommand::clear();
        let vertex_array = self.triangle_data.expect("Should have vertex array by now");
        let triangle_shader = self.triangle_shader.expect("Should have shader by now");

        RenderCommand::bind_shader(triangle_shader);
        RenderCommand::set_shader_uniform_fvec3(triangle_shader, "u_Color", &self.color);
        RenderCommand::draw_indexed(vertex_array);
    }

    fn imgui_update(&mut self, _delta_time: f32, ui: &mut imgui::Ui) {
        ui.window("Hello Triangle")
            .size([300.0, 300.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Primera ventana imgui en proto-ecs");
                let mut triangle_color = self.color.to_array();
                ui.color_picker3("Triangle Color", &mut triangle_color);
                self.color = macaw::Vec3::from_array(triangle_color);
            });
    }

    fn on_event(&mut self, _event: &mut Event) {}
}

fn main() {
    App::initialize();
    WindowManager::init(
        WindowBuilder::new()
            .with_height(720)
            .with_width(720)
            .with_title("Sandbox Testing".to_owned()),
        Platforms::Windows,
    );
    Render::init();

    App::add_layer(Box::new(MyLayer {
        triangle_shader: None,
        triangle_data: None,
        color: macaw::Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    }));

    App::run_application();

    Render::shutdown();
}
