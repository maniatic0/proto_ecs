/// Events are issued by the windowing system, usually in a platform specific manner 
/// but translated to this canonical Event data types to abstract platform-specific APIs


#[derive(Debug)]
pub struct Event {
    handled : bool,
    event_type : Type
}

#[derive(Debug)]
pub enum Type {
    WindowClose, WindowResize, WindowFocus, WindowLostFocus, WindowMoved,
    AppTick, AppUpdate, AppRender, 
    KeyPressed, KeyReleased, KeyTyped,
    MouseButtonPressed, MouseButtonReleased, MouseMoved, MouseScrolled
}

impl Event {
    fn is_handled(&self) -> bool {
        self.handled
    }

    fn make_handled(&mut self) {
        self.handled = true
    }
}
