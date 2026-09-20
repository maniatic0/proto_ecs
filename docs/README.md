# proto-ecs documentation

`proto-ecs` is an experimental Rust game-engine prototype built around an entity-component-system architecture, a staged update loop, and a small OpenGL rendering layer.

## Documents

- [Project overview](overview.md) — repository layout and the main concepts.
- [Runtime architecture](architecture.md) — initialization, frame flow, worlds, and threading.
- [ECS guide](ecs.md) — entities, datagroups, local systems, global systems, and macros.
- [Rendering guide](rendering.md) — windows, OpenGL, render-thread handoff, assets, and materials.
- [Sandbox examples](sandbox.md) — how the example applications use the engine.
- [Build and test status](build-and-test.md) — commands that currently work and known warnings/limitations.

## Quick start

From the repository root:

```powershell
cargo check --manifest-path proto_ecs\Cargo.toml
cargo test --manifest-path proto_ecs\Cargo.toml
cargo check --manifest-path sandbox\Cargo.toml
```

The sandbox applications create graphical Windows/OpenGL windows, so running them requires a suitable desktop environment.
