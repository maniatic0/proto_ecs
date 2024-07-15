use std::time::Instant;


use crate::core::layer::{LayerManager, LayerPtr};
use crate::core::locking::RwLock;
use crate::core::time::Time;
use crate::core::events::Event;
use crate::core::window::{WindowBuilder, WindowPtr};
use crate::data_group::DataGroupRegistry;
use crate::systems::global_systems::GlobalSystemRegistry;
use crate::systems::local_systems::LocalSystemRegistry;
/// This module implements the entire Application workflow.
/// Put any glue code between parts of our application here
use lazy_static::lazy_static;

pub type LayerID = u32;

pub struct App {
    is_initialized: bool,
    time : Time,
    running : bool,
    layer_manager : LayerManager,
    window_ptr : Option<WindowPtr>
}

lazy_static! {
    static ref APP: RwLock<App> = RwLock::new(App::new());
}

impl App {
    fn new() -> Self {
        App {
            is_initialized: false,
            time : Time::new(Instant::now()),
            running: false,
            layer_manager : Default::default(),
            window_ptr : None
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

    pub fn initialize_window(window_builder : WindowBuilder) {
        let mut global_app = APP.write();
        global_app.window_ptr = Some(window_builder.build());
    }

    pub fn is_initialized() -> bool {
        App::get().read().is_initialized
    }

    pub fn get() -> &'static RwLock<App> {
        &APP
    }

    pub fn run_application() {
        // TODO Ask Chris
        // Will we leave this lock on during the entire application? 
        let mut global_app = APP.write();
        println!("Starting to run application!");
        global_app.run();
    }

    pub fn add_layer(layer : LayerPtr) -> LayerID {
        let mut global_app = APP.write();
        global_app.layer_manager.attach_layer(layer)
    }

    pub fn add_overlay(overlay : LayerPtr) -> LayerID {
        let mut global_app = APP.write();
        global_app.layer_manager.attach_overlays(overlay)
    }

    fn init(&mut self) {
        self.is_initialized = true;
        self.running = true;
        self.time = Time::new(Instant::now());
    }

    fn get_window(&mut self) -> &mut WindowPtr {
        self.window_ptr.as_mut().unwrap()
    }
    fn run(&mut self) {
        
        while self.running {

            // Time update
            self.time.step(Instant::now());
            let delta_time = self.time.delta_seconds();
            
            // Event polling
            let mut events = self.get_window().poll_events();
            for event in events.iter_mut() {
                self.on_event(event);
            }

             // If layers were requested in runtime, add them just before the next frame.
            // Must of the time this returns immediately
            self.layer_manager.attach_pending_layers();
            self.layer_manager.attach_pending_overlays();

            for layer in self.layer_manager.layers_iter_mut() {
                layer.layer.update(delta_time);
            }

            self.layer_manager.detach_pending_layers();
            self.layer_manager.detach_pending_overlays();
        }

        // Closing the application, detach all layers
        for layer in self.layer_manager.layers_iter_mut() {
            layer.layer.on_attach();
        }

        for layer in self.layer_manager.overlays_iter_mut() {
            layer.layer.on_attach();
        }
    }

    fn on_event(&mut self, event : &mut Event) {

        for layer in self.layer_manager.layers_iter_mut() {
            layer.layer.on_event(event);
        }

        for layer in self.layer_manager.layers_iter_mut() {
            layer.layer.on_event(event);
        }
    }
}
