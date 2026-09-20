# Core design principles

## Purpose

This engine is designed for games. Runtime performance, predictable frame time, data locality, and control over memory and concurrency are core goals.

The engine is not intended to maximize general-purpose abstraction or eliminate every form of manual management. A design that is unusual or more difficult to use may still be correct if it provides a meaningful performance, memory-layout, or scheduling benefit.

Performance decisions must remain explainable. Every deliberate shortcut should identify:

1. The invariant it relies on.
2. Where that invariant is established.
3. Which code is allowed to rely on it.
4. What happens if it is violated.

## Performance philosophy

### Keep hot paths lean

Per-frame and per-entity execution should avoid unnecessary work, including:

- Repeated validation of data already validated during setup.
- Excessive generality that introduces pointer chasing.
- Unnecessary heap allocation.
- Repeated dynamic dispatch where static or generated dispatch is practical.
- Locks or synchronization on data with a clear single-thread owner.
- Defensive branches that protect against states the engine has already ruled out.

This does not mean removing all checks. It means putting each check where it has the best cost-to-value ratio.

### Avoid accidental pessimizations

Before adding an abstraction, consider its runtime cost:

- Does it add indirection?
- Does it prevent contiguous iteration?
- Does it add synchronization to a single-threaded path?
- Does it cause extra ownership or reference-counting traffic?
- Does it make the compiler less able to inline or optimize?

Abstractions should be justified by safety, clarity, or measurable performance—not added solely because they are more general.

### Measure before redesigning

Performance-sensitive changes should be guided by benchmarks or profiling. The existing Criterion benchmarks should be expanded as important subsystems mature.

Useful measurements include:

- Entity allocation and reuse.
- Entity creation/deletion queues.
- Local-system dispatch.
- Global-system membership updates.
- Transform hierarchy updates.
- Asset preparation and model uploads.
- Render-proxy collection.
- Frame handoff and render-thread latency.

## Validation boundaries

The preferred model is to validate aggressively at non-hot boundaries and keep the runtime path lean.

### Validate during setup and loading

Loading, registration, spawning, and scene preparation should validate:

- Asset formats and required data.
- Datagroup initialization arguments.
- Local/global system dependencies.
- System ordering and cycles.
- Handle validity where handles enter a subsystem.
- Material/shader compatibility.
- Transform hierarchy constraints.
- Render-resource ownership and lifetime requirements.

Validation may happen on a loading thread or preparation thread when possible, so the main/update thread does not pay the cost.

### Trust validated runtime data

After validation produces an engine-ready object, hot code may rely on its documented invariants. For example, a prepared mesh can guarantee that its vertex layout, indices, materials, and GPU-upload metadata are compatible.

The invariant should be represented in the type or documented next to the unsafe/internal operation whenever practical.

### Safe checked paths and unsafe fast paths

When a check is useful for users but expensive in a proven hot path, provide two levels of access:

```rust
allocator.get(handle)           // safe, checked or guarded access
allocator.get_unchecked(handle) // unsafe, caller guarantees validity
```

The unchecked path must be narrow and documented. It should not be used merely to hide an unclear ownership model.

Debug assertions are useful for development diagnostics, but they must not be the only protection against memory unsafety. If invalid release-build input could cause undefined behavior, the safe API must check it or return an error.

## Unsafe code policy

Unsafe code is allowed in core engine code when it provides a meaningful benefit, such as:

- Stable or compact storage.
- Custom allocation.
- Avoiding unnecessary copies.
- Efficient handles and indirection.
- Specialized synchronization.
- Direct platform or GPU integration.

Unsafe code must follow these rules:

1. Keep unsafe blocks as small as possible.
2. Prefer an `unsafe fn` when the caller must uphold a contract.
3. Place a `# Safety` section on unsafe public functions and traits.
4. Add a safety comment beside every non-obvious unsafe block.
5. Document ownership, aliasing, initialization, and thread assumptions.
6. Use `unsafe impl Send`/`Sync` only with a written justification.
7. Provide a safe wrapper for normal user-facing use cases.

An unsafe implementation should make its assumptions easier to audit, not make them invisible.

## Ownership and handles

Handles should generally be small, copyable identifiers containing an index and generation. A handle may outlive the allocator as a value, but it must not be dereferenced after the allocator or resource has been destroyed.

The design must distinguish:

- Handle lifetime — how long the identifier can be stored.
- Allocator lifetime — how long the backing storage exists.
- Resource lifetime — whether the referenced slot is currently live.
- Access lifetime — how long a borrowed read/write view remains valid.

Generation checks protect against stale slot reuse, but they do not by themselves solve aliasing or thread safety.

For mutable resource access, the engine must choose an explicit model:

- Single-threaded access with a borrow/guard API.
- Thread-safe per-entry locks and read/write guards.
- Ownership by one thread with commands or immutable snapshots sent to others.
- A carefully justified unsafe fast path for internal code.

The current generational allocator and its manual `Send`/`Sync` implementations require a dedicated safety contract before being treated as a general concurrent API. In particular, `RefCell` is not a cross-thread synchronization primitive, and returning `&mut V` from `&self` can violate Rust aliasing rules.

## Concurrency and thread ownership

Every shared subsystem should document:

- Which thread owns allocation and destruction.
- Which threads may read it.
- Which threads may mutate it.
- Whether access is lock-free, locked, queued, or snapshot-based.
- Whether operations are immediate or deferred to a stage/frame boundary.

When possible, prefer ownership transfer and message passing over making every object globally synchronized. For example, render-thread ownership of OpenGL resources combined with immutable frame descriptions may be safer and faster than exposing mutable GPU objects to every thread.

Deferred queues are appropriate when structural changes would interfere with iteration. Their processing point must be documented so users know when a create, destroy, parent, or resource command becomes visible.

## Safe user-facing APIs

Non-core engine code should provide safe defaults. Users should not need to understand raw pointers, allocator internals, or synchronization details for ordinary workflows.

Safe APIs should still avoid unnecessary pessimization:

- Validate configuration during loading or spawning.
- Return prepared/validated objects for runtime use.
- Use generated/static dispatch where practical.
- Avoid per-frame allocations for ordinary operations.
- Make ownership and thread restrictions visible in types or documentation.

Advanced users may access lower-level unsafe APIs when they can uphold stronger invariants and need the performance or control. Those APIs should be clearly marked and documented rather than hidden behind undocumented behavior.

## Documentation requirements

Performance-sensitive subsystems should document:

- Their data layout and ownership model.
- Their hot paths.
- Which checks occur during setup versus runtime.
- Which operations are deferred.
- Which unsafe invariants callers must uphold.
- Which thread may access each resource.
- Why a less conventional design was chosen.

This documentation is part of the engine’s design contract. It is intended to help both developers and coding agents preserve the reasoning behind performance-sensitive decisions.

## Decision checklist

Before adding or changing a core abstraction, ask:

- Is this on a hot path?
- Can validation happen earlier or off the main thread?
- Does this add allocation, indirection, locking, or reference counting?
- Can the invariant be represented in a type or prepared object?
- Is the unsafe boundary small and documented?
- Is there a safe default API for users?
- Is there an explicit owner and thread for the data?
- Do benchmarks or profiling support the tradeoff?
