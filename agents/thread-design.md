# Thread and frame design

## Purpose

The core unit of work in the engine is a frame. A frame identifies a period in which the authoritative game state is updated and then published to other systems, such as rendering and audio.

This document describes the intended development model for engine threads and frame synchronization. It is a design guideline, not a description of every detail currently implemented.

## Thread roles

### Main Thread

The Main Thread owns the authoritative game-state frame. It drives the simulation loop and is responsible for:

- Advancing simulation time.
- Updating the entity system and gameplay systems.
- Applying input, physics, network, and background-system results at defined boundaries.
- Dispatching parallel work required by the current frame.
- Publishing a committed frame snapshot to downstream systems.

The Main Thread must not progress past a synchronization point until all work required for the current phase of the frame is complete.

### OS Event Thread

The OS Event Thread communicates with the operating system. It handles tasks such as:

- Processing keyboard, mouse, window, and platform events.
- Keeping the application responsive to the OS.
- Receiving close, resize, and focus notifications.

On platforms that require OS/window work on the application thread, the OS Event Thread may be the same thread as the Main Thread.

OS events should be queued and associated with a simulation frame or input sequence. The Main Thread consumes them at a defined point rather than allowing events to mutate game state asynchronously.

### Render System

The Render System consumes committed frame data produced by the Main Thread. It runs on one or more Render Threads depending on the graphics API and platform.

Some APIs, such as OpenGL, may require a single thread to own the graphics context. Other APIs, such as Vulkan, may support several worker threads while still having a controlled submission/ownership model.

The Render System may be behind the Main Thread by a bounded number of frames, normally no more than two or three. This allows rendering and simulation to overlap.

Render frames are presentation work, so the renderer may skip obsolete snapshots when a newer snapshot is available. The Main Thread may be blocked by backpressure when the Render System is too far behind, preventing unbounded frame-data growth.

### Audio System

The Audio System consumes audio events and state associated with committed Main Thread frames. It should process the same logical simulation frame as the corresponding downstream systems where practical, but its device/mixer clock is independent from rendering.

Audio must not pause merely because rendering skips or delays a frame. It should use buffering, interpolation, or other audio-specific techniques to maintain continuous output.

### Physics System

The Physics System runs in lockstep with the Main Thread at the simulation level. Physics work may execute in parallel with gameplay updates, but the two systems exchange data at explicit synchronization points.

### Network System

Network I/O runs asynchronously where possible. Network state is consumed and applied to the authoritative simulation at a defined Main Thread frame boundary. Outgoing messages should be generated from a committed simulation state or explicit network command stream.

### Background Threads

Background Threads handle work that should not block Main Thread progress, including:

- Asset loading and parsing.
- Resource preparation and decompression.
- File I/O.
- Tooling or diagnostics.
- Other deferred jobs that do not need immediate frame results.

Background work publishes completion messages, prepared resources, or commands. It should not directly mutate live authoritative game state.

## Frame work categories

The Main Thread may dispatch two broad categories of work.

### Frame-bound work

Frame-bound work must complete before the current frame can advance past its phase or publish its downstream snapshot.

Examples:

- Parallel entity updates.
- Gameplay systems required for the current simulation step.
- Physics synchronization and result application.
- Transform propagation needed by rendering.
- Building the render/audio frame snapshot.

This work should use the engine’s frame job system or thread pool. The Main Thread waits at the relevant barrier before continuing.

### Deferred/background work

Deferred work may finish after the current frame and must not prevent the Main Thread from progressing.

Examples:

- Requesting a model load.
- Reading and parsing asset files.
- Decompressing resource data.
- Preparing data for a future entity spawn.
- Non-critical diagnostics or cache maintenance.

Deferred jobs communicate through queues or futures. A typical asset flow is:

```text
Main Thread requests asset
    ↓
Background Thread loads and validates asset
    ↓
Background Thread publishes prepared result
    ↓
Main/Render Thread adopts result at a safe boundary
    ↓
Future entity or frame can use the asset
```

The request itself may be initiated by a frame, but the asset does not become visible to the simulation until its completion is applied at a defined boundary.

## Physics synchronization

Physics needs a synchronization point inside the simulation frame, before render-frame publication.

A frame should conceptually proceed as follows:

```text
1. Begin simulation frame N
2. Consume input/network/background results ready for N
3. Publish current gameplay state to Physics
4. Physics advances using the current physics input
5. Synchronization point: Main consumes Physics results
6. Gameplay reacts to Physics results
7. Run remaining gameplay/transform work
8. Publish render/audio data for frame N
9. Downstream systems consume the committed frame
```

The exact order of steps 3–7 may evolve, but the important guarantees are:

- Physics results are posted to the Main Thread at an explicit sync point.
- Gameplay has an opportunity to react to those results during the same simulation frame or according to a documented one-frame-latency policy.
- Physics receives the latest approved simulation updates before its next step.
- Physics results and gameplay reactions are applied before render data is published.

Physics must not directly mutate ECS state while gameplay systems or other frame jobs are iterating it. Results should be exchanged through buffers, commands, or a controlled ownership handoff.

## Frame snapshots and ownership

Each published downstream snapshot should carry a monotonically increasing frame ID. A snapshot should contain immutable data or data whose ownership has been transferred to the consuming system.

The intended ownership model is:

```text
Main Thread owns authoritative mutable simulation state
    ↓ publish
Downstream systems own/read committed snapshots
    ↓ results/commands
Main Thread applies approved results at frame boundaries
```

Handles, pointers, and references into live ECS state must not be stored in a downstream snapshot unless their lifetime and mutation rules explicitly guarantee validity until consumption finishes.

## Scheduling principles

- Work required for a frame belongs to that frame’s job graph and must have a defined completion barrier.
- Work that can complete later should be dispatched without blocking the Main Thread.
- Structural mutations should be deferred until the ECS reaches a safe processing point.
- Cross-thread communication should use bounded queues, explicit ownership transfer, immutable snapshots, or command buffers.
- Every synchronization point should document what data becomes visible after it.
- Downstream systems may lag by a bounded number of frames; unbounded queues are not acceptable for frame data.

## Determinism and frame identity

Frame IDs should be used to associate:

- Input events.
- Physics inputs and results.
- Network commands and received state.
- Render snapshots.
- Audio events.
- Background job requests and completion messages.

This makes latency and ordering visible and provides a foundation for deterministic replay, debugging, and future rollback networking.

## Summary

The engine uses a pipelined frame model:

```text
OS events ────────┐
                  ▼
             Main Thread
        authoritative simulation
          ├── frame jobs
          ├── Physics sync point
          └── publish frame N
                    │
          bounded downstream pipeline
             ┌──────┴──────┐
             ▼             ▼
       Render System   Audio System
        frame N-k       frame N-k

Background Threads prepare resources and return commands/results asynchronously.
```
