use std::time::Instant;

use crate::core::layer::{LayerManager, LayerPtr};
use crate::core::locking::RwLock;
use crate::core::rendering::render_thread::RenderThread;
use crate::core::time::Time;
use crate::core::windowing::events::{Event, Type};
use crate::core::windowing::window_manager::WindowManager;
use crate::data_group::DataGroupRegistry;
use crate::entities::entity_system::{EntitySystem, WorldID};
use crate::systems::global_systems::GlobalSystemRegistry;
use crate::systems::local_systems::LocalSystemRegistry;
/// This module implements the entire Application workflow.
/// Put any glue code between parts of our application here
use lazy_static::lazy_static;

pub type LayerID = u32;

pub struct App {
    is_initialized: bool,
    time: Time,
    running: bool,
    pub(crate) layer_manager: LayerManager,
    world: WorldID
}

lazy_static! {
    static ref APP: RwLock<App> = RwLock::new(App::new());
}

impl App {
    fn new() -> Self {
        App {
            is_initialized: false,
            time: Time::new(Instant::now()),
            running: false,
            layer_manager: Default::default(),
            world: 0
        }
    }

    /// Initialize internal systems (like datagroup registry).
    pub fn initialize() {
        let mut global_app = APP.write();
        if global_app.is_initialized {
            // Already initialized
            return;
        }

        println!("Initializing app!");

        // Put any initialization logic here, mind the expected initialization order.
        debug_assert!(
            !DataGroupRegistry::get_global_registry()
                .read()
                .is_initialized(),
            "DataGroupRegistry should not initialize before app"
        );
        DataGroupRegistry::initialize();

        // Local system should update after datagroups since they rely on datagroup ids for
        // initialization
        debug_assert!(
            !LocalSystemRegistry::get_global_registry()
                .read()
                .is_initialized(),
            "LocalSystemRegistry should not initialize before app"
        );
        LocalSystemRegistry::initialize();

        // Global systems can initialize at any point
        GlobalSystemRegistry::initialize();

        global_app.init();
    }

    pub fn is_initialized() -> bool {
        App::get().read().is_initialized
    }

    pub fn get() -> &'static RwLock<App> {
        &APP
    }

    pub fn run_application() {
        println!("Waiting for render thread...");
        while !RenderThread::is_started() {}
        // TODO Ask Chris
        // Will we leave this lock on during the entire application?
        let mut global_app = APP.write();
        println!("Starting to run application!");
        global_app.run();
    }

    pub fn add_layer(layer: LayerPtr) -> LayerID {
        let mut global_app = APP.write();
        global_app.layer_manager.attach_layer(layer)
    }

    pub fn add_overlay(overlay: LayerPtr) -> LayerID {
        let mut global_app = APP.write();
        global_app.layer_manager.attach_overlays(overlay)
    }

    fn init(&mut self) {
        self.is_initialized = true;
        self.running = true;
        self.time = Time::new(Instant::now());
        let es = EntitySystem::get();
        self.world = es.create_world();
    }

    fn run(&mut self) {
        while self.running {
            // Time update
            self.time.step(Instant::now());
            let delta_time = self.time.delta_seconds();

            // Event polling
            {
                let mut window_manager = WindowManager::get().write();
                window_manager.get_window_mut().handle_window_events(self);
            }

            // If layers were requested in runtime, add them just before the next frame.
            // Must of the time this returns immediately
            self.layer_manager.attach_pending_layers();
            self.layer_manager.attach_pending_overlays();

            // Update anything the user might want to do at the layer level
            for layer in self.layer_manager.layers_iter_mut() {
                layer.layer.update(delta_time);
            }
            for layer in self.layer_manager.overlays_iter_mut() {
                layer.layer.update(delta_time);
            }

            // Update the entity system 
            // TODO Compute actual fixed delta time
            // TODO Compute delta time with f64 precision
            {
                let es = EntitySystem::get();
                es.step(delta_time as f64, -1.0); // -1 so it crashes if you use it
            }

            self.layer_manager.detach_pending_layers();
            self.layer_manager.detach_pending_overlays();
            {
                let mut window_manager = WindowManager::get().write();
                window_manager.get_window_mut().on_update();
            }
        }

        // Closing the application, detach all layers
        for layer in self.layer_manager.layers_iter_mut() {
            layer.layer.on_detach();
        }

        for layer in self.layer_manager.overlays_iter_mut() {
            layer.layer.on_detach();
        }
    }

    /// This function should be called by the window manager before swaping buffers.
    /// This is necessary because the window manager only has access to the `ui` object
    /// when it is about to swap buffers. The ui object cannot be created in the
    /// main loop and get a reference to it later in the window manager, due to how
    /// imgui-rs works. Check [crate::core::platform::winit_window::WinitWindow]'s implementation
    /// of the [crate::core::window::Window] trait, particularly `handle_window_events`
    pub(crate) fn run_imgui(&mut self, ui: &mut imgui::Ui) {
        for layer in self.layer_manager.layers_iter_mut() {
            layer.layer.imgui_update(self.time.delta_seconds(), ui);
        }
        for layer in self.layer_manager.overlays_iter_mut() {
            layer.layer.imgui_update(self.time.delta_seconds(), ui);
        }
    }

    pub fn on_event(&mut self, event: &mut Event) {
        // Event is handled, ignore it.
        // Handled events are no propagated later in the event stack
        if event.is_handled() {
            return;
        }

        self.handle_event(event);
        for layer in self.layer_manager.layers_iter_mut() {
            layer.layer.on_event(event);
        }

        for layer in self.layer_manager.layers_iter_mut() {
            layer.layer.on_event(event);
        }
    }

    fn handle_event(&mut self, event: &mut Event) {
        if let Type::WindowClose = event.get_type() {
            self.running = false;
        }
    }
}
