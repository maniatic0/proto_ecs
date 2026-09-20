# Codebase design audit

## Scope

This report reviews the repository against:

- [Core design principles](core-design.md)
- [Thread and frame design](thread-design.md)
- [Remaining warnings](remaining-warnings.md)
- [Current render-thread concerns](render-thread-concerns.md)

The review was performed as four parallel read-only inspections covering unsafe memory/ownership, thread/frame behavior, public APIs/documentation, and performance. No source code was changed as part of the audit.

## Executive summary

The engine’s high-level direction is coherent: the Main Thread owns simulation, ECS changes are deferred, rendering is separated onto a render thread, and work is intended to be split between frame-bound jobs and background preparation.

The current implementation has not yet made several of those boundaries enforceable. The most important concerns are:

1. The generational allocator exposes mutable references from shared access and advertises thread safety while containing `RefCell`.
2. `EntityPtr` exposes raw, non-owning lifetime and concurrency assumptions through an ordinary public `Deref` API.
3. OpenGL/backend types are manually marked `Send`/`Sync` even though the intended model is render-thread ownership.
4. Render-frame publication is controlled by a boolean and window-side behavior rather than an explicit frame-completion protocol.
5. The renderer currently has a single shared frame slot, not the documented bounded N-frame pipeline.
6. Several intentional performance shortcuts—raw pointer ECS iteration, boxed entity storage, dynamic datagroups, global state—are not documented with their invariants or tradeoffs.

The performance philosophy is viable, but the current code often exposes unsafe assumptions through safe-looking APIs. The next design work should establish ownership and synchronization contracts before adding more parallel subsystems.

## Priority 0 — correctness and soundness

### 1. Generational allocator API is unsound

Location: [handle.rs:66-140](../proto_ecs/src/core/utils/handle.rs:66)

The allocator contains `RefCell`, but manually implements `Send` and `Sync`. More seriously:

```rust
pub fn get(&self, key: K) -> &mut V
```

This permits overlapping mutable references from shared access. The temporary `RefMut` is dropped before the returned reference is used, so `RefCell` does not protect the escaped reference.

The validity check is also only a `debug_assert!`, meaning invalid handles may reach unsafe code in release builds.

Classification: confirmed undefined-behavior and concurrency risk.

Recommended direction:

- Make safe mutable access return a guard.
- Or require `&mut self`.
- Provide a separate, narrowly documented `unsafe get_unchecked` for trusted hot paths.
- Keep validity checks on safe release-build paths.
- Remove or justify the manual `Send`/`Sync` implementations.

Detailed discussion: [remaining-warnings.md](remaining-warnings.md).

### 2. `EntityPtr` exposes unbounded lifetime assumptions

Location: [entity_allocator.rs:45-60](../proto_ecs/src/entities/entity_allocator.rs:45)

`EntityPtr` is a raw, non-owning pointer marked `Send` and `Sync` ([entity_allocator.rs:240-241](../proto_ecs/src/entities/entity_allocator.rs:240)). Generation checks help detect slot reuse, but they do not prevent this race:

```text
Thread A checks is_live()
Thread B frees and reuses the entity
Thread A dereferences the pointer
```

The entity lock protects entity contents, not the allocator slot’s lifetime. Dereference uses `debug_assert!`, so release builds do not establish a safe failure mode for stale pointers.

The engine needs to choose and document one model:

- Pointers are valid only during a stable frame/stage phase.
- Destruction is deferred until all pointer users finish.
- Public access is checked/guarded, with an internal unchecked path.
- Ownership is transferred to jobs and reclamation waits for completion.

Classification: confirmed potential use-after-free/data-race risk unless a stronger owner-thread rule exists.

### 3. OpenGL thread-safety contract is contradictory

Locations:

- [opengl_render_backend.rs:25-43](../proto_ecs/src/core/platform/opengl/opengl_render_backend.rs:25)
- [opengl_shader.rs:13-15](../proto_ecs/src/core/platform/opengl/opengl_shader.rs:13)
- [opengl_vertex_array.rs:11-13](../proto_ecs/src/core/platform/opengl/opengl_vertex_array.rs:11)
- [render_api.rs:22-24](../proto_ecs/src/core/rendering/render_api.rs:22)

The implementation comments indicate that the OpenGL context is thread-local, but the backend, shaders, and vertex arrays are manually marked `Send`/`Sync`. Public render commands are available through globally shared state.

If the intended rule is “all OpenGL operations happen on the Render Thread,” the type/API design should not advertise arbitrary cross-thread use. Main-thread code should communicate through handles, immutable frame data, or render commands.

Classification: high-priority correctness and API-contract risk.

## Priority 1 — frame and synchronization design

### 4. Frame publication is not tied to simulation completion

`RenderGS` writes the next frame description in [rendering.rs:59-116](../proto_ecs/src/systems/engine/rendering.rs:59), but the explicit publication call is commented out at [rendering.rs:119](../proto_ecs/src/systems/engine/rendering.rs:119).

The active call is associated with the window path ([winit_window.rs:83-102](../proto_ecs/src/core/platform/winit_window.rs:83)), which leaves the contract unclear:

- Is a frame published when ECS completes?
- When the window swaps buffers?
- When ImGui processing completes?
- Can presentation advance without a new simulation frame?

This does not match the intended model where Main Thread completion publishes a committed frame snapshot.

Classification: design/implementation mismatch.

Detailed discussion: [render-thread-concerns.md](render-thread-concerns.md).

### 5. Current renderer is a one-slot handoff, not a bounded N-frame pipeline

Locations:

- [render_thread.rs:39-69](../proto_ecs/src/core/rendering/render_thread.rs:39)
- [render_thread.rs:159-162](../proto_ecs/src/core/rendering/render_thread.rs:159)

The current design has one shared `FrameDesc`, one render-local `FrameDesc`, and a boolean completion flag. It has no frame IDs, slot states, queue depth, obsolete-frame policy, or defined backpressure.

This cannot yet represent “renderer is two or three simulation frames behind” in a precise way.

Recommended protocol:

```text
Free → MainWriting → Ready(frame_id) → Rendering → Free
```

Use two or three bounded slots, explicit frame IDs, and a waitable backpressure mechanism.

### 6. Busy-waiting is both a performance and ownership smell

Locations:

- [render_thread.rs:123-128](../proto_ecs/src/core/rendering/render_thread.rs:123)
- [app.rs:86-87](../proto_ecs/src/app.rs:86)

The Render Thread spins while waiting for a frame, and application startup spins while waiting for render initialization. This can consume a CPU core and does not express the intended wait/notify or shutdown protocol.

Use a condition variable, semaphore, channel, or equivalent. The primitive should support both frame publication and clean shutdown.

### 7. Multiple worlds have no render ownership policy

Worlds run in parallel ([entity_system.rs:1007-1009](../proto_ecs/src/entities/entity_system.rs:1007)), while each `RenderGS` writes the one global frame description. The lock prevents simultaneous writes but does not define how worlds combine; whichever world writes last can replace the previous frame.

Choose one policy:

- One designated render world.
- One snapshot per world.
- A Main Thread merge phase.
- A separate render-submission stage.

### 8. ECS unsafe disjointness is not documented sufficiently

The stage runner bypasses entity locks using raw pointers ([entity_system.rs:630-634](../proto_ecs/src/entities/entity_system.rs:630)), and transform traversal does similar work ([entity.rs:455-469](../proto_ecs/src/entities/entity.rs:455)).

The optimization may be valid, but the code should document:

- Why two jobs cannot reach the same entity.
- How stage/global-system vectors remain disjoint.
- Why parent/child recursion cannot overlap incorrectly.
- Why structural queues cannot invalidate entries during a stage.
- What prevents destruction while a raw pointer is in use.

The best shape is a narrow unsafe helper with a precise safety contract, surrounded by debug-only invariant checks when stage lists are built.

## Priority 2 — lifecycle and API contracts

### 9. Application initialization/shutdown order is implicit

`App`, `WindowManager`, and `Render` are separate global systems. Public APIs do not clearly state the required order, whether shutdown is mandatory, or whether reinitialization is supported.

Relevant locations:

- [app.rs:43-91](../proto_ecs/src/app.rs:43)
- [render.rs:53-78](../proto_ecs/src/core/rendering/render.rs:53)
- [render_api.rs:127-149](../proto_ecs/src/core/rendering/render_api.rs:127)

Document this as a lifecycle state machine at minimum. A future explicit engine context could make invalid ordering harder to express.

### 10. `App::run_application()` holds the global write lock for the full loop

Location: [app.rs:85-93](../proto_ecs/src/app.rs:85)

The application acquires the global `App` write lock and retains it throughout the run loop. This may be intentional single-owner execution, but it means any other access through `App::get()` can block for the entire application lifetime.

This needs either documentation as an intentional ownership rule or redesign.

### 11. Layer thread affinity and deferred visibility are unclear

`Layer` requires `Send + Sync` ([layer.rs:14-27](../proto_ecs/src/core/layer.rs:14)), but callbacks appear to run synchronously on the application thread. The trait should state its thread affinity rather than implying concurrent access if none exists.

Attach/detach operations are queued ([layer.rs:35-47](../proto_ecs/src/core/layer.rs:35)) and processed at frame boundaries, but the public contract does not define when IDs become active, what happens when operations cancel each other, or whether callbacks can detach themselves.

Also, `overlays_iter` takes `&mut self` despite being read-only ([layer.rs:127-130](../proto_ecs/src/core/layer.rs:127)).

### 12. Event dispatch appears incorrect

`App::on_event()` iterates regular layers twice ([app.rs:179-193](../proto_ecs/src/app.rs:179)) instead of apparently iterating regular layers and overlays. It also checks `event.is_handled()` before dispatch but does not stop propagation when a layer handles the event.

This should be confirmed and fixed before documenting event semantics.

### 13. Registry and unsafe transform APIs rely on debug-only validation

Registry lookups use `debug_assert!` and then index vectors directly, for example [data_group.rs:204-205](../proto_ecs/src/data_group.rs:204). Unsafe transform accessors have incomplete safety contracts ([entity.rs:283-316](../proto_ecs/src/entities/entity.rs:283)).

The project can intentionally keep trusted hot-path APIs, but it should separate:

- Safe checked public access.
- Explicitly unsafe prevalidated access.
- The setup phase where IDs and dependencies become trusted.

## Priority 3 — performance and locality

### 14. Entity access has substantial indirection

A typical path can be:

```text
DashMap → EntityPtr → Box<EntityEntry> → RwLock<Entity>
        → Vec<Box<dyn DataGroup>> → dynamic cast
```

This may be justified by flexible composition, but it conflicts with the project’s locality goals unless measured. The existing cached datagroup indices are a useful direction. Benchmark before replacing the design, then consider dense typed storage for hot datagroups while retaining the flexible path for uncommon data.

### 15. Structural deletion and command processing may cause frame spikes

Deletion scans stage/global-system/entity vectors linearly and repeatedly acquires locks. Command processing also allocates temporary vectors at stage boundaries.

These may be acceptable if structural changes are rare. If not, consider reverse indices, reusable scratch buffers, batching, and a threshold before dispatching small jobs to Rayon.

### 16. Global systems are effectively serial

Global systems are processed sequentially and take several locks. This may be an intentional safety/performance tradeoff, but it should be documented. If parallel global systems are needed later, introduce explicit read/write and dependency metadata rather than assuming they can run concurrently.

### 17. Runtime rendering performs avoidable work

The renderer currently performs or may perform the following in hot paths:

- HashMap model lookups per proxy.
- String-based material parameter lookup.
- Per-object updates of frame-wide camera uniforms.
- First-visible model packing and GPU upload.
- Dynamic allocations during frame/render preparation.

These should be measured before optimization. Likely improvements include prepared render data, numeric uniform locations, material/model grouping, frame-wide uniform setup, and explicit resource-upload commands.

### 18. Asset loading is synchronous

`Render::get_or_load_model()` calls model parsing synchronously ([render.rs:119-128](../proto_ecs/src/core/rendering/render.rs:119); [models.rs:42-72](../proto_ecs/src/core/assets_management/models.rs:42)). This can block gameplay on filesystem I/O, parsing, and allocation.

Keep a synchronous path for startup/loading screens if useful, but add an asynchronous request path for active gameplay. Background workers should validate and prepare CPU assets; Main/Render Threads should adopt them at defined boundaries.

## API typing and memory-layout concerns

### 19. Resource handles are all aliases of the same type

Locations: [render_api.rs:11-14](../proto_ecs/src/core/rendering/render_api.rs:11)

Shader, vertex-buffer, index-buffer, and vertex-array handles are all aliases of `Handle`. A handle from one allocator can therefore be passed to an API for another resource kind without compile-time rejection.

Distinct newtypes or a typed generic handle would likely provide zero-cost protection while preserving compact runtime representation.

### 20. Entity pointer reconstruction relies on layout assumptions

Location: [entity_allocator.rs:168-182](../proto_ecs/src/entities/entity_allocator.rs:168)

`EntityEntry::from_ptr` reconstructs the containing object using a size difference. The assumptions about field order, representation, alignment, and final-field placement should be asserted or documented beside the unsafe code.

### 21. Heterogeneous memory block has an incomplete drop/safety contract

Location: [het_single_mem_block.rs:45-67](../proto_ecs/src/core/het_single_mem_block.rs:45)

The documentation does not fully define alignment, initialization, movement, sharing, pointer validity, or destruction. `Drop` deallocates raw memory but does not appear to drop values constructed inside it. This may be intentional, but callers need an explicit destruction contract.

### 22. Sandbox vertex conversion is overly generic and undocumented

Location: [sandbox/main.rs:47-58](../sandbox/src/main.rs:47)

`any_as_f32_slice` reinterprets arbitrary `T` as an `f32` slice. It should either have a complete safety contract or be replaced by a constrained/explicit vertex representation. This is non-core code, so a safe user-facing implementation should be preferred.

## Positive findings

- The staged ECS model and deferred structural commands are a good basis for parallel simulation.
- Local-system function-pointer dispatch is a good performance-oriented choice.
- Generation counters show awareness of stale-handle problems.
- The render-thread separation matches the intended long-term architecture.
- Existing tests cover registration, system ordering, allocator reuse, hierarchy, and expected failures.
- Macro-generated registration keeps the user-facing ECS model compact.

## Recommended next steps

### Phase 1: establish contracts

1. Decide whether allocator/entity allocation and destruction are Main Thread-owned.
2. Define valid `EntityPtr` access windows.
3. Remove or justify manual `Send`/`Sync` implementations.
4. Define render-thread ownership of OpenGL objects.
5. Define frame publication, frame IDs, and bounded in-flight snapshots.

### Phase 2: make contracts visible

1. Add safety comments and `# Safety` sections.
2. Add safe checked APIs and explicit unsafe fast paths.
3. Add typed resource-handle newtypes.
4. Document lifecycle and thread affinity for App, layers, registries, assets, and render resources.
5. Clarify stage/frame command visibility.

### Phase 3: measure and optimize

1. Benchmark ECS indirection and lock overhead.
2. Benchmark structural deletion and command processing.
3. Move runtime asset preparation off the Main Thread.
4. Prepare material/model state for efficient rendering.
5. Optimize only after the frame and ownership protocols are stable.

## Conclusion

The engine has a promising performance-oriented architecture, but several safety and ownership boundaries are currently implicit. The highest-risk pattern is not the use of unsafe code itself; it is that unsafe assumptions are exposed through APIs that look safe or thread-independent.

The project can retain manual memory management, raw pointers, specialized allocators, and low-level rendering code. It needs to make the contracts explicit, put validation at the correct boundaries, and ensure that every unsafe fast path has a safe or clearly documented counterpart.
