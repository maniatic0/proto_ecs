# Runtime architecture

## Startup sequence

The normal startup sequence is:

1. `App::initialize()` initializes the datagroup, local-system, and global-system registries.
2. `WindowManager::init(...)` creates the platform window. The current implementation supports Windows through Winit/Glutin.
3. `Render::init()` creates shared render state and starts the render thread.
4. The application attaches one or more `Layer` values.
5. `App::run_application()` waits for the render thread, then enters the main loop.

The application creates a default ECS world during `App::initialize()`/application initialization. Layers are attached through queues and receive `on_attach()` when processed at a frame boundary.

## Main-loop flow

Each frame, `App`:

1. Advances `Time` and obtains delta time.
2. Polls window events and forwards them to the application/layers.
3. Attaches pending layers and overlays.
4. Calls `update(delta_time)` on layers and overlays.
5. Calls `EntitySystem::step(delta_time, fixed_delta_time)`.
6. Detaches requested layers and overlays.
7. Lets the window finish/update the frame.

The ECS step updates world timing and processes every configured stage. Before and after each stage it processes queued world/entity commands. Worlds are run in parallel through a Rayon thread pool.

## Layers

`Layer` is the application-facing callback interface:

- `on_attach()` — initialize layer resources.
- `on_detach()` — clean up layer resources.
- `update()` — per-frame application logic.
- `on_event()` — input/window event handling.
- `imgui_update()` — optional UI work.

Layers and overlays are managed separately, although both participate in updates and UI processing.

## Concurrency model

The implementation uses several concurrent containers and synchronization primitives:

- Rayon for parallel world/stage execution.
- DashMap for world/entity maps.
- SCC queues for deferred commands.
- Custom/global read-write locks around shared engine state.
- A separate render thread communicating through a shared frame description.

Many operations are intentionally deferred to stage or frame boundaries so entity collections are not structurally modified while systems iterate them.

## Key source files

- [Application loop](../proto_ecs/src/app.rs)
- [Entity system](../proto_ecs/src/entities/entity_system.rs)
- [Layer manager](../proto_ecs/src/core/layer.rs)
- [Timing](../proto_ecs/src/core/time.rs)
