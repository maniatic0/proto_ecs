use proto_ecs::prelude::*;

struct MyLayer;

impl Layer for MyLayer {
    fn on_attach(&mut self) {}

    fn on_detach(&mut self) {}

    fn update(&mut self, delta_time: f32) {}

    fn on_event(&mut self, event: &mut Event) {}
}

fn main() {
    App::initialize();
    WindowManager::init(
        WindowBuilder::new()
            .with_height(300)
            .with_width(600)
            .with_title("Sandbox Testing".to_owned()),
        Platforms::Windows,
    );

    App::add_layer(Box::new(MyLayer));

    App::run_application();
}
