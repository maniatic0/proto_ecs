/// Window trait definitions
/// 
/// This file provides the traits that should be provided by any platform-specific window implementation.
/// 
/// Note that there no implementation nor storage in this file. For window instances management, see [window_manager]
use std::rc::Rc;
use std::any::Any;

use crate::prelude::App;

use super::casting::CanCast;

pub struct WindowBuilder {
    pub width : u32,
    pub height : u32,
    pub title : String,
}

pub type WindowPtr = Box<dyn WindowDyn>;

pub trait WindowDyn : Send + Sync + CanCast + HasImguiContext{
    fn get_width(&self) -> u32;

    fn get_heigth(&self) -> u32;

    fn set_vsync(&mut self, is_vsync_active : bool);

    fn get_vsync(&self) -> bool;

    fn get_native_window(&self) -> Rc<dyn Any>;

    fn handle_window_events(&mut self, app : &mut App);

    fn on_update(&mut self);
}

pub (crate) trait HasImguiContext {
    fn get_imgui_context(&self) -> &imgui::Context;
}

/// Every platform-specific window implementation should implement this trait. 
pub trait Window : WindowDyn {
    fn create(window_builder : WindowBuilder) -> WindowPtr;
}

impl WindowBuilder {
    pub fn new() -> Self {
        return WindowBuilder {
            title : "Proto ECS".to_owned(),
            height : 300,
            width: 300
        }
    }

    pub fn with_width(mut self, width : u32) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height : u32) -> Self {
        self.height = height;
        self
    }

    pub fn with_title(mut self, title : String) -> Self {
        self.title = title;
        self
    }
}
