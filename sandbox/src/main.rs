use proto_ecs::prelude::*;

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

    fn on_event(&mut self, event: &Event) {
        println!("Handling event")
    }
}

fn main() {
    App::initialize();

    App::add_layer(Box::new(MyLayer));

    App::run_application();
}
