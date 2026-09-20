# Experimental ECS engine — suggested follow-up work

## Priority 0: correctness and safety

These should be addressed before relying on the engine for a larger project.

### Make render-frame synchronization explicit

The render thread currently coordinates frame handoff with atomic flags and busy-waiting. The rendering global system also contains a commented-out `RenderThread::next_frame_updated()` call.

Goals:

- Define exactly when a frame is ready for consumption.
- Guarantee that the render thread cannot repeatedly consume stale data.
- Replace spin loops with a condition variable, channel, or another blocking primitive.
- Add a small synchronization test or diagnostic mode.

Relevant files:

- `proto_ecs/src/core/rendering/render_thread.rs`
- `proto_ecs/src/systems/engine/rendering.rs`

### Audit unsafe ownership and thread-safety assumptions

The entity allocator and rendering code use raw pointers, `MaybeUninit`, and manual `Send`/`Sync` implementations.

Goals:

- Document the invariants for `EntityPtr` and allocator entry lifetime.
- Minimize the scope of unsafe blocks.
- Replace manual `Send`/`Sync` implementations where safe abstractions are possible.
- Run stress tests under tools such as Miri, ThreadSanitizer-compatible configurations, or a dedicated concurrency test harness where feasible.

Relevant files:

- `proto_ecs/src/entities/entity_allocator.rs`
- `proto_ecs/src/entities/entity_system.rs`
- `proto_ecs/src/app.rs`
- `sandbox/src/main.rs`

### Test concurrent mutation more aggressively

Add tests that create, destroy, parent, and reparent entities while systems are running across multiple worlds and stages.

Important cases include:

- Entity creation during a stage.
- Entity destruction during a stage.
- Reuse of allocator slots after deletion.
- Global systems loading/unloading while entities change membership.
- World destruction or merging while commands are queued.

## Priority 1: simplify project structure and lifecycle

### Add a root Cargo workspace

Create a root `Cargo.toml` with `proto_ecs` and `sandbox` as workspace members. This should make dependency resolution, testing, and common commands more predictable.

Potential commands after the change:

```powershell
cargo check --workspace
cargo test --workspace
```

### Reduce global initialization coupling

The engine currently depends on global registries and ordered initialization through `App::initialize()`.

Possible direction:

- Keep registration collection static if desired.
- Move runtime state into an explicit `Engine` or `AppContext`.
- Make initialization phases visible in the public API.
- Return initialization errors instead of relying primarily on assertions and panics.

### Improve lifecycle cleanup

Complete or clarify resource cleanup for:

- Layers.
- Models and GPU model data.
- Materials and shaders.
- Windows and render threads.
- Datagroups and global systems.

Each resource should have a clear owner and shutdown point.

## Priority 2: improve diagnostics and usability

### Improve system dependency errors

Replace broad panics with structured errors where possible. Include:

- System name.
- Missing datagroup.
- Entity name/ID.
- World ID.
- Stage number.

Detect and report cyclic local-system dependencies with the actual cycle rather than only a generic message.

### Add tracing/logging

Replace scattered `println!` calls with a logging/tracing layer. Useful targets include:

- Entity creation/deletion.
- System registration and execution.
- World lifecycle events.
- Render-thread frame timing.
- Asset loading and GPU uploads.

### Add user documentation examples

Document a minimal custom application containing:

1. A custom datagroup.
2. A local system.
3. An entity spawn description.
4. A layer.
5. Optional rendering.

This will make the macro-generated API much easier to adopt.

## Priority 3: performance and feature growth

### Measure before optimizing

Add benchmarks for:

- Entity allocation/free/reuse.
- Entity creation and deletion queues.
- Local-system dispatch.
- Global-system membership updates.
- Transform hierarchy updates.
- Render-proxy collection.

Use the existing Criterion benchmark setup as the starting point.

### Revisit entity storage layout

The current pointer/lock-per-entity model is flexible but may become expensive at scale. Compare it against archetype, sparse-set, or column-oriented storage for frequently iterated datagroups.

Do this only after profiling real workloads; the current design may be appropriate for the intended use case.

### Complete rendering milestones

Potential next rendering features:

- Reliable frame synchronization.
- Window resize propagation.
- Explicit scene/frame boundaries.
- Resource unloading.
- Texture/material support.
- Better camera selection.
- Render passes and visibility filtering.

## Small cleanup items

- Fix lifetime elision warnings in layer iterator signatures.
- Remove or use the unused model variable.
- Decide whether unused unloading code should be implemented or removed.
- Replace function-pointer equality assertions in tests with behavior-based checks.
- Fix the duplicate layer event iteration if it is unintended.
- Add CI that runs `cargo fmt --check`, `cargo clippy`, `cargo test`, and workspace checks.
