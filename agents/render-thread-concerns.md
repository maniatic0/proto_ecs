# Current render-thread concerns

This document records implementation concerns in the current renderer. They are separate from the intended thread/frame design in [thread-design.md](thread-design.md), which describes the target architecture rather than current defects or incomplete paths.

## Frame publication is incomplete

The rendering global system builds the next `FrameDesc`, but the call that marks the frame as ready appears to be commented out:

```rust
// RenderThread::next_frame_updated();
```

The render thread waits for the shared `last_frame_finished` flag to become false before consuming a frame. If no publisher clears that flag, the render thread can continue waiting and render stale or no frame data.

### Recommended direction

Define an explicit publication operation that:

1. Assigns a monotonically increasing frame ID.
2. Marks the completed frame slot as ready.
3. Wakes the render thread.
4. Applies backpressure when the maximum number of in-flight frames is reached.

The publication point should happen only after all render-relevant simulation, physics, and transform work for that frame is complete.

## Busy-waiting

`RenderThread::run()` currently checks the frame-finished state in a loop. This can consume a CPU core while waiting for the Main Thread.

### Recommended direction

Use a condition variable, semaphore, channel, or equivalent waitable primitive. The render thread should sleep when no frame is ready and be notified when a frame is published.

The synchronization primitive should also support shutdown so the render thread cannot remain blocked after `RenderThread::stop()`.

## Shared frame storage needs explicit ownership

The current implementation swaps a shared `FrameDesc` between the Main Thread and Render Thread. This can work, but the ownership contract needs to be explicit:

- The producer must not modify a frame while the renderer reads it.
- The renderer must finish reading a frame before it becomes writable again.
- A model/material handle in a frame must remain valid until rendering finishes.
- Resource unloading must not invalidate a resource referenced by an in-flight frame.

### Recommended direction

Use a bounded frame-slot state machine, for example:

```text
Free → Writing → Ready → Reading → Free
```

Each slot should carry a frame ID and ownership state. A two- or three-slot ring buffer would support pipelining without allowing unbounded frame-data growth.

## Backpressure and frame skipping

The intended renderer may be behind the Main Thread by a small bounded number of frames. The implementation should define:

- The maximum number of in-flight frames.
- Whether old unconsumed frames may be discarded.
- When the Main Thread blocks because the renderer is behind.
- Whether the renderer always consumes the newest frame or preserves every frame.

For ordinary visual rendering, dropping obsolete snapshots is usually acceptable. Simulation frames should never be dropped by the renderer; only presentation snapshots may be skipped.

## Render-resource lifetime

Models and materials referenced by render proxies need a lifetime policy. In particular, unloading must account for:

- CPU-side model storage.
- GPU vertex/index buffers.
- Materials and shaders.
- Entities that still reference the resource.
- Frame snapshots already queued for rendering.

A resource should not be freed until no live entity, queued command, or in-flight frame can reference it.

## Suggested implementation order

1. Add explicit frame IDs and a clear publication point.
2. Replace the busy-wait with a waitable synchronization primitive.
3. Introduce bounded frame slots with ownership states.
4. Add backpressure and an explicit frame-skipping policy.
5. Define resource-retirement rules for queued and in-flight frames.
6. Add tests or diagnostics for stale frames, shutdown, dropped snapshots, and renderer lag.
