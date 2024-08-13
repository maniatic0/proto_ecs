/// This module implements management of the window instance
use lazy_static::lazy_static;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::platform::{winit_window, Platforms};

use super::window::{Window, WindowBuilder, WindowPtr};

pub struct WindowManager {
    window: Option<WindowPtr>,
    platform: Platforms,
}

impl WindowManager {
    fn new() -> Self {
        WindowManager {
            window: None,
            platform: Platforms::None,
        }
    }

    pub fn init(window_builder: WindowBuilder, platform: Platforms) {
        let mut window_manager = WINDOW_MANAGER.write();
        window_manager.init_instance(window_builder, platform);
    }

    pub fn get() -> &'static RwLock<WindowManager> {
        &WINDOW_MANAGER
    }

    pub fn get_platform() -> Platforms {
        Self::get().read().platform
    }

    fn init_instance(&mut self, window_builder: WindowBuilder, platform: Platforms) {
        match platform {
            Platforms::Windows => {
                self.window = Some(winit_window::WinitWindow::create(window_builder));
                self.platform = platform;
            }
            _ => panic!("Unimplemented platform"),
        }
    }

    pub fn get_window(&self) -> &WindowPtr {
        self.window.as_ref().unwrap()
    }

    pub fn get_window_mut(&mut self) -> &mut WindowPtr {
        self.window.as_mut().unwrap()
    }
}

lazy_static! {
    static ref WINDOW_MANAGER: RwLock<WindowManager> = RwLock::new(WindowManager::new());
}
