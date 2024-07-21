use std::num::NonZeroU32;
use std::time::Duration;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface};
use proto_ecs::core::events;
use proto_ecs::core::events::Event;
/// Winit implementation of the window trait object.
use proto_ecs::core::window::{Window, WindowDyn, WindowPtr};
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::LogicalSize;
use winit::event::{MouseButton, MouseScrollDelta};
use winit::event_loop::EventLoop;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Window as winit_Window, WindowBuilder};

use crate::core::keys::Keycode;
use crate::prelude::App;

pub struct WinitWindow {
    width: u32,
    height: u32,
    title: String,
    window: winit_Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    gl_context: glow::Context,
    event_loop: EventLoop<()>,
    use_vsync: bool,
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

    fn handle_window_events(&mut self, app: &mut App) {
        self.event_loop
            .pump_events(Some(Duration::ZERO), |event, event_loop| {
                app.on_event(&mut Event::from(event));
            });
    }

    fn get_vsync(&self) -> bool {
        self.use_vsync
    }

    fn set_vsync(&mut self, is_vsync_active: bool) {
        if self.use_vsync == is_vsync_active {
            return;
        }

        // TODO Actually turn on/off vsync
        self.use_vsync = is_vsync_active;
    }

    fn get_native_window(&self) -> std::rc::Rc<dyn std::any::Any> {
        unimplemented!("TODO Don't know how to return a pointer to the internal window handle")
    }

    fn on_update(&mut self) {
        self.window.request_redraw();
        self.surface
            .swap_buffers(&self.context)
            .expect("Error swaping buffers");
    }
}

impl Window for WinitWindow {
    fn create(window_builder: crate::core::window::WindowBuilder) -> WindowPtr {
        let props = window_builder;
        let window_builder = WindowBuilder::new()
            .with_title(props.title.clone())
            .with_inner_size(LogicalSize::new(props.width, props.height))
            .with_decorations(true);

        let event_loop = winit::event_loop::EventLoop::new()
            .expect("Could not build event loop for winit window");

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

        Box::new(WinitWindow {
            width: props.width,
            height: props.height,
            title: props.title,
            window,
            surface,
            context,
            gl_context,
            event_loop,
            use_vsync: true,
        })
    }
}

fn glow_context(context: &PossiblyCurrentContext) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function_cstr(|s| context.display().get_proc_address(s).cast())
    }
}

impl From<winit::event::Event<()>> for Event {
    fn from(value: winit::event::Event<()>) -> Self {
        match value {
            winit::event::Event::WindowEvent { event, .. } => return Event::from(event),
            _ => Event::new(events::Type::Unknown),
        }
    }
}

impl From<winit::event::WindowEvent> for Event {
    fn from(value: winit::event::WindowEvent) -> Self {
        match value {
            winit::event::WindowEvent::Resized(size) => Event::new(events::Type::WindowResize {
                new_width: size.width,
                new_height: size.height,
            }),
            winit::event::WindowEvent::CloseRequested => Event::new(events::Type::WindowClose),
            winit::event::WindowEvent::Focused(focused) => {
                if focused {
                    return Event::new(events::Type::WindowFocus);
                } else {
                    return Event::new(events::Type::WindowLostFocus);
                }
            }
            winit::event::WindowEvent::Moved(new_pos) => Event::new(events::Type::WindowMoved {
                new_x: new_pos.x,
                new_y: new_pos.y,
            }),
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let button = mouse_btn_to_proto_ecs_btn(button);
                return Event::new(events::Type::MouseButtonEvent {
                    button,
                    state: events::KeyState::from(state),
                });
            }
            winit::event::WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = match delta {
                    MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
                    MouseScrollDelta::LineDelta(x, y) => (x, y),
                };
                return Event::new(events::Type::MouseScrolled { x, y });
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);
                // Note that x,y is the new position relative to the top left corner of the screen
                return Event::new(events::Type::MouseMoved { x, y });
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                return Event::new(events::Type::KeyEvent {
                    key: Keycode::from(event.key_without_modifiers()),
                    state: events::KeyState::from(event.state),
                    repeat: event.repeat,
                })
            }
            _ => Event::new(events::Type::Unknown), // An event not recognized by our system
        }
    }
}

impl From<winit::keyboard::Key> for Keycode {
    fn from(_value: winit::keyboard::Key) -> Self {
        Keycode::Unknown
    }
}

impl From<winit::event::ElementState> for events::KeyState {
    fn from(value: winit::event::ElementState) -> Self {
        match value {
            winit::event::ElementState::Pressed => events::KeyState::Pressed,
            winit::event::ElementState::Released => events::KeyState::Released,
        }
    }
}

fn mouse_btn_to_proto_ecs_btn(btn: MouseButton) -> events::MouseButton {
    match btn {
        MouseButton::Back => events::MouseButton::Back,
        MouseButton::Forward => events::MouseButton::Forward,
        MouseButton::Left => events::MouseButton::Left,
        MouseButton::Right => events::MouseButton::Right,
        MouseButton::Other(e) => events::MouseButton::Other(e),
        MouseButton::Middle => events::MouseButton::Middle,
    }
}
