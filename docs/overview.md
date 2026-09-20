# Project overview

The repository contains two Rust packages:

| Package | Purpose |
| --- | --- |
| `proto_ecs` | The reusable engine and ECS library, including platform, windowing, rendering, asset, and macro support. |
| `sandbox` | Example applications that exercise the engine. |

There is no workspace manifest at the repository root. Each package is built by passing its own manifest to Cargo.

## Important directories

- `proto_ecs/src/app.rs` — application lifecycle and main loop.
- `proto_ecs/src/entities/` — entity allocation, entity state, spawn descriptions, worlds, and the entity system.
- `proto_ecs/src/data_group.rs` — datagroup trait, initialization metadata, and registry.
- `proto_ecs/src/systems/` — local/global system registries and engine systems.
- `proto_ecs/src/core/` — timing, layers, locks, platform integration, rendering, assets, and utilities.
- `proto_ecs/ecs_macros/` — procedural macros used to register engine/user types.
- `proto_ecs/src/tests/` — registration, ECS, and system behavior tests.
- `sandbox/resources/` — sample OBJ/FBX models and texture assets.

## Design idea

The engine separates user-facing behavior into layers and ECS systems:

- Layers handle application-level updates, events, and ImGui.
- Datagroups hold entity data.
- Local systems update datagroups on individual entities.
- Global systems process groups of entities and engine-wide concerns.
- The renderer consumes data assembled by a rendering global system.
