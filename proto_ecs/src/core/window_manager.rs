
/// This module implements management of the window instance 
use lazy_static::lazy_static;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::platform::winit_window;

use super::window::{Platforms, Window, WindowBuilder, WindowPtr};

pub struct WindowManager {
    window : Option<WindowPtr>
}

impl WindowManager {
    fn new() -> Self {
        WindowManager{
            window: None
        }
    }

    pub fn init(window_builder : WindowBuilder, platform : Platforms) {
        let mut window_manager = WINDOW_MANAGER.write();
        window_manager.init_instance(window_builder, platform);
    }

    pub fn get() -> &'static RwLock<WindowManager> {
        &WINDOW_MANAGER
    }

    fn init_instance(&mut self, window_builder : WindowBuilder, platform : Platforms) {
        match platform {
            Platforms::Windows => {
                self.window = Some(winit_window::WinitWindow::create(window_builder))
            }
        }
    }
}

lazy_static!{
    static ref WINDOW_MANAGER : RwLock<WindowManager> = RwLock::new(WindowManager::new());
}