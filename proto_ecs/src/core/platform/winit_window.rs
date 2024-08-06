use std::num::NonZeroU32;
use std::time::Duration;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::PossiblyCurrentGlContext;
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface};
use proto_ecs::core::events;
use proto_ecs::core::events::Event;
use proto_ecs::core::window::{Window, WindowDyn, WindowPtr};
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::LogicalSize;
use winit::event::{MouseButton, MouseScrollDelta};
use winit::event_loop::EventLoop;
use winit::keyboard::NamedKey;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Window as winit_Window, WindowBuilder};

use crate::core::casting::CanCast;
use crate::core::keys::Keycode;
use crate::prelude::App;

#[derive(CanCast)]
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
            .pump_events(Some(Duration::ZERO), |event, _event_loop| {
                match event {
                    winit::event::Event::WindowEvent { event: winit::event::WindowEvent::RedrawRequested,..} =>  {
                        if !self.context.is_current() {
                            self.context.make_current(&self.surface).expect("Could not make this the current context");
                        }
                        self.surface
                            .swap_buffers(&self.context)
                            .expect("Error swaping buffers in winit window");
                    },
                    _ => ()
                };
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

        // TODO Check that this changes vsync state properly
        self.use_vsync = is_vsync_active;
        if self.use_vsync {
            // Waits for the next event, most likely a "RedrawRequested" from the OS
            self.event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait)
        }
        else {
            // Runs another loop regardless of whether there's a new event or not
            self.event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        }
    }

    fn get_native_window(&self) -> std::rc::Rc<dyn std::any::Any> {
        unimplemented!("TODO Don't know how to return a pointer to the internal window handle")
    }

    fn on_update(&mut self) {
        self.window.request_redraw();
    }
}

impl WinitWindow {
    pub fn get_glow_context(&self) -> glow::Context {
        glow_context(&self.context)
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
        let mut result = Box::new(WinitWindow {
            width: props.width,
            height: props.height,
            title: props.title,
            window,
            surface,
            context,
            gl_context,
            event_loop,
            use_vsync: false,
        });

        result.set_vsync(true);
        result
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
                let button = events::MouseButton::from(button);
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
        match _value {
            winit::keyboard::Key::Named(nkey) => match nkey {
                NamedKey::Enter => Keycode::Enter,
                NamedKey::ArrowDown => Keycode::Down,
                NamedKey::ArrowRight => Keycode::Right,
                NamedKey::ArrowLeft => Keycode::Left,
                NamedKey::ArrowUp => Keycode::Up,
                NamedKey::Shift => Keycode::LShift,
                NamedKey::Control => Keycode::LCtrl,
                NamedKey::Escape => Keycode::Escape,
                NamedKey::CapsLock => Keycode::CapsLock,
                NamedKey::Alt => Keycode::LAlt,
                NamedKey::F1 => Keycode::F1,
                NamedKey::F2 => Keycode::F2,
                NamedKey::F3 => Keycode::F3,
                NamedKey::F4 => Keycode::F4,
                NamedKey::F5 => Keycode::F5,
                NamedKey::F6 => Keycode::F6,
                NamedKey::F7 => Keycode::F7,
                NamedKey::F8 => Keycode::F8,
                NamedKey::F9 => Keycode::F9,
                NamedKey::F10 => Keycode::F10,
                NamedKey::F11 => Keycode::F11,
                NamedKey::F12 => Keycode::F12,
                NamedKey::NumLock => Keycode::NumLockClea,
                NamedKey::Insert => Keycode::Insert,
                NamedKey::Delete => Keycode::Delete,
                NamedKey::PageDown => Keycode::PageDown,
                NamedKey::PageUp => Keycode::PageUp,
                NamedKey::Home => Keycode::Home,
                NamedKey::End => Keycode::End,
                NamedKey::Backspace => Keycode::Backspace,
                NamedKey::Space => Keycode::Space,
                NamedKey::Tab => Keycode::Tab,
                _ => Keycode::Unknown,
            },
            winit::keyboard::Key::Character(c) => {
                if c == "a" {
                    Keycode::A
                } else if c == "b" {
                    Keycode::B
                } else if c == "c" {
                    Keycode::C
                } else if c == "d" {
                    Keycode::D
                } else if c == "e" {
                    Keycode::E
                } else if c == "f" {
                    Keycode::F
                } else if c == "g" {
                    Keycode::G
                } else if c == "h" {
                    Keycode::H
                } else if c == "i" {
                    Keycode::I
                } else if c == "j" {
                    Keycode::J
                } else if c == "k" {
                    Keycode::K
                } else if c == "l" {
                    Keycode::L
                } else if c == "m" {
                    Keycode::M
                } else if c == "n" {
                    Keycode::N
                } else if c == "o" {
                    Keycode::O
                } else if c == "p" {
                    Keycode::P
                } else if c == "q" {
                    Keycode::Q
                } else if c == "r" {
                    Keycode::R
                } else if c == "s" {
                    Keycode::S
                } else if c == "t" {
                    Keycode::T
                } else if c == "u" {
                    Keycode::U
                } else if c == "v" {
                    Keycode::V
                } else if c == "w" {
                    Keycode::W
                } else if c == "x" {
                    Keycode::X
                } else if c == "y" {
                    Keycode::Y
                } else if c == "z" {
                    Keycode::Z
                } else if c == "0" {
                    Keycode::Num0
                } else if c == "1" {
                    Keycode::Num1
                } else if c == "2" {
                    Keycode::Num2
                } else if c == "3" {
                    Keycode::Num3
                } else if c == "4" {
                    Keycode::Num4
                } else if c == "5" {
                    Keycode::Num5
                } else if c == "6" {
                    Keycode::Num6
                } else if c == "7" {
                    Keycode::Num7
                } else if c == "8" {
                    Keycode::Num8
                } else if c == "9" {
                    Keycode::Num9
                } else if c == "_" {
                    Keycode::Underscore
                } else if c == "-" {
                    Keycode::Minus
                } else if c == "+" {
                    Keycode::Plus
                } else if c == "=" {
                    Keycode::Equals
                } else if c == "<" {
                    Keycode::Less
                } else if c == ">" {
                    Keycode::Greater
                } else if c == "." {
                    Keycode::Period
                } else if c == "," {
                    Keycode::Comma
                } else if c == ":" {
                    Keycode::Colon
                } else if c == ";" {
                    Keycode::Semicolon
                } else if c == "[" {
                    Keycode::LeftBracket
                } else if c == "]" {
                    Keycode::RightBracke
                } else if c == "(" {
                    Keycode::LeftParen
                } else if c == ")" {
                    Keycode::RightParen
                } else if c == "{" {
                    Keycode::KpLeftBrace
                } else if c == "}" {
                    Keycode::KpRightBrace
                } else if c == "`" {
                    Keycode::Backquote
                } else if c == "'" {
                    Keycode::Quote
                } else if c == "\"" {
                    Keycode::QuoteDouble
                } else if c == "/" {
                    Keycode::Slash
                } else if c == "\\" {
                    Keycode::Backslash
                } else if c == "?" {
                    Keycode::Question
                } else if c == "!" {
                    Keycode::Exclaim
                } else if c == "&" {
                    Keycode::Ampersand
                } else if c == "%" {
                    Keycode::Percent
                } else if c == "$" {
                    Keycode::Dollar
                } else if c == "#" {
                    Keycode::Hash
                } else if c == "@" {
                    Keycode::At
                } else if c == "*" {
                    Keycode::Asterisk
                } else if c == "^" {
                    Keycode::Power
                } else {
                    Keycode::Unknown
                }
            }

            _ => Keycode::Unknown,
        }
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

impl From<MouseButton> for events::MouseButton {
    fn from(value: MouseButton) -> Self {
        match value {
            MouseButton::Back => events::MouseButton::Back,
            MouseButton::Forward => events::MouseButton::Forward,
            MouseButton::Left => events::MouseButton::Left,
            MouseButton::Right => events::MouseButton::Right,
            MouseButton::Other(e) => events::MouseButton::Other(e),
            MouseButton::Middle => events::MouseButton::Middle,
        }
    }
}
