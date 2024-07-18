use std::num::NonZeroU32;
use std::time::Duration;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface};
/// Winit implementation of the window trait object.
use proto_ecs::core::window::{WindowDyn, Window, WindowPtr};
use proto_ecs::core::events::Event;
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Window as winit_Window, WindowBuilder};

use crate::prelude::App;

pub struct WinitWindow {
    width : u32,
    height : u32,
    title : String,
    window: winit_Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    gl_context: glow::Context,
    event_loop: EventLoop<()>,
    use_vsync : bool,
}

// TODO work on a safe implementation for these traits 
unsafe impl Send for WinitWindow {}
unsafe impl Sync for WinitWindow {}

impl WindowDyn for WinitWindow {
    fn get_heigth(&self) -> u32 {
        self.height
    }

    fn get_width(&self) -> u32 {
        self.width
    }

    fn handle_events(&mut self, app : &mut App ) {
        self.event_loop.pump_events(Some(Duration::ZERO), |event, event_loop|{
            app.on_event(&mut to_event(event));
        });

    }

    fn get_vsync(&self) -> bool {
        self.use_vsync
    }

    fn set_vsync(&mut self, is_vsync_active : bool) {
        if self.use_vsync == is_vsync_active {
            return
        }

        self.use_vsync = is_vsync_active;
    }

    fn get_native_window(&self) -> std::rc::Rc<dyn std::any::Any> {
        unimplemented!("TODO Don't know how to return a pointer to the internal window handle")
    }

    fn on_update(&mut self) {
        self.window.request_redraw();
        self.surface.swap_buffers(&self.context).expect("Error swaping buffers");
    }
}

impl Window for WinitWindow {
    fn create(window_builder : crate::core::window::WindowBuilder) -> WindowPtr {
        let props = window_builder;
        let window_builder = WindowBuilder::new()
            .with_title(props.title.clone())
            .with_inner_size(LogicalSize::new(props.width, props.height))
            .with_decorations(true);

        let event_loop =
            winit::event_loop::EventLoop::new().expect("Could not build event loop for winit window");

        // Window creation
        let (window, cfg) = glutin_winit::DisplayBuilder::new()
            .with_window_builder(Some(window_builder))
            .build(&event_loop, ConfigTemplateBuilder::new(), |mut configs| {
                configs.next().unwrap()
            })
            .expect("Failed to create Winit Window");

        let window = window.expect("Failed to create Winit Window");

        // Context Creation
        let context_attrs = ContextAttributesBuilder::new().build(Some(window.raw_window_handle()));

        let context = unsafe {
            cfg.display()
                .create_context(&cfg, &context_attrs)
                .expect("Failed to create OpenGL Winit context")
        };

        let surface_attrs = SurfaceAttributesBuilder::<WindowSurface>::new()
            .with_srgb(Some(true))
            .build(
                window.raw_window_handle(),
                NonZeroU32::new(props.width).unwrap(),
                NonZeroU32::new(props.height).unwrap(),
            );
        let surface = unsafe {
            cfg.display()
                .create_window_surface(&cfg, &surface_attrs)
                .expect("Failed to create OpenGL surface for window")
        };

        let context = context
            .make_current(&surface)
            .expect("Error making OpenGL context the current context");

        let gl_context = glow_context(&context);

        Box::new(
            WinitWindow{
                width: props.width,
                height: props.height,
                title: props.title,
                window, 
                surface, 
                context, 
                gl_context,
                event_loop,
                use_vsync: true
            }
        )
    }
}

fn glow_context(context: &PossiblyCurrentContext) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function_cstr(|s| context.display().get_proc_address(s).cast())
    }
}

fn to_event(event : winit::event::Event<()>) -> Event {
    unimplemented!("TODO buscar una forma de traducir los eventos nativos a eventos abstraidos")
}