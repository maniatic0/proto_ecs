use std::rc::Rc;
use std::any::Any;
use proto_ecs::core::events::Event;

pub struct WindowBuilder {
    properties : WindowProperties,
    builder_fn : WindowBuilderFn
}

pub struct WindowProperties {
    pub width : u32,
    pub height : u32,
    pub title : String,
}

pub type WindowPtr = Box<dyn Window>;
pub type WindowBuilderFn = fn (WindowProperties) -> WindowPtr;

pub trait Window : Send + Sync {

    fn get_width(&self) -> u32;

    fn get_heigth(&self) -> u32;

    fn set_vsync(&mut self, is_vsync_active : bool);

    fn get_vsync(&self);

    fn get_native_window(&self) -> Rc<dyn Any>;

    fn poll_events(&mut self) -> Vec<Event>;
}

impl WindowBuilder {
    pub fn new() -> Self {
        return WindowBuilder {
            properties: WindowProperties {
                title : "Proto ECS".to_owned(),
                height : 300,
                width: 300
            },
            builder_fn : default_window
        }
    }

    pub fn with_width(mut self, width : u32) -> Self {
        self.properties.width = width;
        self
    }

    pub fn with_height(mut self, height : u32) -> Self {
        self.properties.height = height;
        self
    }

    pub fn with_title(mut self, title : String) -> Self {
        self.properties.title = title;
        self
    }

    pub fn with_builder_fn(mut self, builder_fn : WindowBuilderFn) -> Self {
        self.builder_fn = builder_fn;
        self
    }

    pub fn build(self) -> WindowPtr {
        (self.builder_fn)(self.properties)
    }
}

/// Return a winit window with the provided implementation
pub fn default_window(window_props : WindowProperties) -> WindowPtr {
    unimplemented!("Not yet implemented");
}