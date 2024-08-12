use glam;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::render::buffer::{
    create_index_buffer, create_vertex_buffer, BufferElement, BufferLayout,
};
use proto_ecs::core::render::render_api::RenderCommand;
use proto_ecs::core::render::shader::{create_shader, ShaderDataType, ShaderPtr};
use proto_ecs::core::render::vertex_array::{create_vertex_array, VertexArrayPtr};
use proto_ecs::prelude::*;
use imgui;

struct MyLayer {
    triangle_shader: Option<RwLock<ShaderPtr>>,
    triangle_data: Option<RwLock<VertexArrayPtr>>,
    color : glam::Vec3,
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
    position: glam::Vec2,
    color: glam::Vec3,
}

// TODO We need a better way to cast custom data types to f32 arrays to send data to the GPU
unsafe fn any_as_f32_slice<T: Sized>(p: &T) -> &[f32] {
    ::core::slice::from_raw_parts((p as *const T) as *const f32, ::core::mem::size_of::<T>())
}
impl Layer for MyLayer {
    fn on_attach(&mut self) {
        self.triangle_shader = Some(RwLock::new({
            let mut shader = create_shader(
                &"Example Triangle".to_string(),
                &VERTEX_SRC.to_string(),
                &FRAGMENT_SRC.to_string(),
            )
            .expect("Could not create triangle shader");
            shader.add_uniform(&"u_Color".into(), ShaderDataType::Float3).expect("Should be able to add this uniform");
            shader
        }
        ));

        static VERTEX_DATA: [VertexData; 3] = [
            VertexData {
                position: glam::vec2(-0.5, -0.5),
                color: glam::vec3(1.0, 0.0, 0.0),
            },
            VertexData {
                position: glam::vec2(0.0, 0.5),
                color: glam::vec3(0.0, 1.0, 0.0),
            },
            VertexData {
                position: glam::vec2(0.5, -0.5),
                color: glam::vec3(0.0, 0.0, 1.0),
            },
        ];

        // Create a buffer for this triangle data
        let mut vbo = create_vertex_buffer(unsafe { any_as_f32_slice(&VERTEX_DATA) });
        vbo.set_layout(BufferLayout::from_elements(vec![
            BufferElement::new("a_Position".into(), ShaderDataType::Float2, false),
            BufferElement::new("a_Color".into(), ShaderDataType::Float3, false),
        ]));
        let index_buffer = create_index_buffer(&[0, 1, 2]);
        let mut vao = create_vertex_array();
        vao.set_vertex_buffer(vbo);
        vao.set_index_buffer(index_buffer);

        self.triangle_data = Some(RwLock::new(vao));
        println!("Triangle data intialized!");
    }

    fn on_detach(&mut self) {}

    fn update(&mut self, delta_time: f32) {
        RenderCommand::set_clear_color(glam::vec4(1.0, 0.5, 0.5, 1.0));
        RenderCommand::clear();
        let vertex_array = self
            .triangle_data
            .as_ref()
            .expect("Should have vertex array by now")
            .read();
        let triangle_shader = self
            .triangle_shader
            .as_ref()
            .expect("Should have shader by now")
            .read();
        triangle_shader.bind();
        triangle_shader.set_uniform_fvec3(&"u_Color".into(), &self.color);
        RenderCommand::draw_indexed(&vertex_array);
    }

    fn imgui_update(&mut self, delta_time: f32, ui : &mut imgui::Ui) {
        ui.window("Hello Triangle")
            .size([300.0, 300.0], imgui::Condition::FirstUseEver)
            .build(||{
                ui.text("Primera ventana imgui en proto-ecs");
                let mut triangle_color = [self.color.x, self.color.y, self.color.z];
                ui.color_picker3("Triangle Color", &mut triangle_color);
                self.color.x = triangle_color[0];
                self.color.y = triangle_color[1];
                self.color.z = triangle_color[2];
            });
    }

    fn on_event(&mut self, event: &mut Event) {}
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
        color: glam::Vec3 { x: 1.0, y: 1.0, z: 1.0 }
    }));

    App::run_application();
}
