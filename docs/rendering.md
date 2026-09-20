# Rendering guide

## Rendering components

Rendering is split into:

- Window/platform creation.
- A render API and OpenGL backend.
- CPU-side model/material management.
- A dedicated render thread.
- ECS global systems that turn entities into renderable frame data.

The current platform path is Windows with Winit, Glutin, OpenGL, and Glow. ImGui is integrated into the window/update path.

## Render-thread handoff

`RenderThread` owns the render-side state. The main/ECS side writes a shared `FrameDesc` containing:

- Render proxies.
- The active camera.

A render proxy references a model, material, transform, and world position. The render thread swaps in the next frame description, uploads models to the GPU if needed, binds shaders/material parameters, applies camera and transform matrices, and issues indexed draws.

## Engine rendering systems

The built-in rendering systems include:

- `MeshRenderer` datagroup — associates models and materials with an entity.
- `CameraDG` datagroup — stores camera data.
- `CameraGS` — selects/configures a camera.
- `RenderGS` — collects mesh entities into render proxies.

The rendering system is staged late in the frame so transforms and other gameplay logic can update before render data is collected.

## Assets and materials

`Render::get_or_load_model(...)` loads model data through `ModelManager`. `Render::create_material(...)` creates a material using a shader handle and shader parameter map.

The sandbox includes OBJ, FBX, and texture assets under [sandbox/resources](../sandbox/resources).

## Relevant source files

- [Render state](../proto_ecs/src/core/rendering/render.rs)
- [Render thread](../proto_ecs/src/core/rendering/render_thread.rs)
- [Render API](../proto_ecs/src/core/rendering/render_api.rs)
- [Engine rendering systems](../proto_ecs/src/systems/engine/rendering.rs)
- [Window manager](../proto_ecs/src/core/windowing/window_manager.rs)
