/// Events are issued by the windowing system, usually in a platform specific manner 
/// but translated to this canonical Event data types to abstract platform-specific APIs

use proto_ecs::core::keys::Keycode;

#[derive(Debug)]
pub struct Event {
    handled : bool,
    event_type : Type
}

#[derive(Debug)]
pub enum Type {
    WindowClose, WindowResize{new_width: u32, new_height: u32}, WindowFocus, WindowLostFocus, WindowMoved{new_x: i32, new_y: i32},
    // These events are rised by our app. Still not sure where or how to trigger them
    AppTick, AppUpdate, AppRender, 
    KeyEvent{key : Keycode, state : KeyState, repeat : bool},
    MouseButtonEvent{button : MouseButton, state : KeyState}, MouseMoved{x: f32, y: f32}, MouseScrolled{x: f32, y: f32},
    Unknown
}


#[derive(Debug)]
pub enum KeyState {
    Pressed,
    Released,
    Repeat
}

#[derive(Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl Event {
    pub fn is_handled(&self) -> bool {
        self.handled
    }

    pub fn make_handled(&mut self) {
        self.handled = true
    }

    pub fn new(event_type : Type) -> Self {
        return Event {
            handled : false,
            event_type 
        }
    }

    pub fn get_type(&self) -> &Type {
        &self.event_type
    }
}
