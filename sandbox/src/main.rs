use proto_ecs::{core::window::{Platforms, WindowBuilder}, prelude::*};

struct MyLayer;

impl  Layer for MyLayer {
    fn on_attach(&mut self) {
        println!("Attaching my layer!")
    }

    fn on_detach(&mut self) {
        println!("Detaching my layer")
    }

    fn update(&mut self, delta_time: f32) {
        println!("Updating updating")
    }

    fn on_event(&mut self, event: &mut Event) {
        println!("Handling event")
    }
}

fn main() {
    App::initialize();
    WindowManager::init(WindowBuilder::new(), Platforms::Windows);

    App::add_layer(Box::new(MyLayer));

    App::run_application();
}
