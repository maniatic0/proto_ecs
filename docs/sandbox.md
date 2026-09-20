# Sandbox examples

The sandbox package contains two binaries.

## Triangle example

[`sandbox/src/main.rs`](../sandbox/src/main.rs) creates a `MyLayer` that:

1. Creates a simple vertex/fragment shader.
2. Builds a vertex buffer, index buffer, and vertex array for a triangle.
3. Clears the screen and draws the triangle every update.
4. Displays an ImGui color picker for the triangle color.

It starts the engine with a 720×720 Windows window titled `Sandbox Testing`.

## Model-rendering example

[`sandbox/src/bin/example_render.rs`](../sandbox/src/bin/example_render.rs) demonstrates the more complete ECS/rendering path:

1. Initializes the application, window, and renderer.
2. Loads the default shader and creates a material.
3. Loads `resources/Rocket/Rocket.obj`.
4. Spawns a model entity with `Transform`, `MeshRenderer`, and `RenderGS`.
5. Spawns a camera entity with `CameraDG` and `CameraGS`.
6. Runs a local system that rotates the model each frame.

The example expects to be launched with its working directory arranged so `./resources/Rocket/Rocket.obj` resolves from the sandbox directory.

## Typical commands

```powershell
cargo check --manifest-path sandbox\Cargo.toml
cargo run --manifest-path sandbox\Cargo.toml
cargo run --manifest-path sandbox\Cargo.toml --bin example_render
```

The last two commands require a Windows/OpenGL desktop environment and will open a window.
