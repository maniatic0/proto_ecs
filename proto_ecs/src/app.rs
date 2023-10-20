use lazy_static::lazy_static;

/// This module implements the entire Application workflow.
/// Put any glue code between parts of our application here
use crate::data_group::DataGroupRegistry;
use crate::local_systems::LocalSystemRegistry;
use parking_lot::RwLock;

pub struct App {
    is_initialized: bool,
}

lazy_static! {
    static ref APP: RwLock<App> = RwLock::new(App::new());
}

impl App {
    fn new() -> Self {
        App {
            is_initialized: false,
        }
    }

    /// Initialize internal systems (like datagroup registry).
    pub fn initialize() {
        let mut global_app = APP.write();
        assert!(!global_app.is_initialized, "App got double initialized!");

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

        global_app.init();
    }

    pub fn is_initialized() -> bool {
        App::get().read().is_initialized
    }

    pub fn get() -> &'static RwLock<App> {
        &APP
    }

    fn init(&mut self) {
        self.is_initialized = true;
    }
}
