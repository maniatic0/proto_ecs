# ECS guide

## Entities and worlds

`EntitySystem` owns multiple `World` instances. A world contains:

- An entity map keyed by `EntityID`.
- Iteration vectors for all entities and per-stage entities.
- Entity creation/deletion and hierarchy queues.
- Loaded global systems.
- Per-global-system entity lists.
- The currently selected camera.

Entities are created from `EntitySpawnDescription`. Creation is deferred and the entity becomes visible to the relevant stage at a safe processing point.

## Entity allocation

`EntityAllocator` stores entity entries in reusable slots. Each slot has a generation counter. `EntityPtr` stores both a pointer and generation, allowing the engine to detect use of a slot after it has been freed and reused.

Entity IDs are separate from allocator slots. IDs are monotonically allocated by the entity system, while allocator memory is reused.

## Datagroups

Datagroups are component-like data owned by entities. A datagroup implements `DataGroup` and is registered with `register_datagroup!`.

A datagroup registration supplies:

- A factory function.
- A stable name/CRC and generated ID metadata.
- Its initialization style: no initialization, no arguments, required arguments, or optional arguments.

`EntitySpawnDescription` validates that every datagroup has the correct initialization data. It also validates system dependencies in debug builds.

The built-in transform datagroup is in [transform_datagroup.rs](../proto_ecs/src/entities/transform_datagroup.rs).

## Local systems

Local systems operate on datagroups attached to one entity. They declare their datagroup dependencies and stages using `register_local_system!`.

The macro generates typed stage functions plus an internal adapter. The registry topologically sorts `before`/`after` relationships and assigns execution IDs, so local systems can execute deterministically.

Example shape:

```rust
register_local_system! {
    ModelRotatorLS,
    dependencies = (Transform),
    stages = (0),
    before = ()
}
```

## Global systems

Global systems operate over the set of entities that requested them. They are appropriate for engine-wide behavior such as rendering or camera selection.

Global systems declare:

- Their stages.
- Required datagroups.
- Their lifetime policy.

The available lifetime behavior includes always-loaded systems and systems loaded/unloaded when entities require them. The world tracks subscriber counts to decide when `WhenRequired` systems can be unloaded.

## Macro registration

The `ecs_macros` crate generates registration and casting boilerplate. Registration functions are collected before application initialization; `App::initialize()` consumes those registrations and freezes/sorts the global registries.

Relevant code:

- [Datagroup API](../proto_ecs/src/data_group.rs)
- [Local systems](../proto_ecs/src/systems/local_systems.rs)
- [Global systems](../proto_ecs/src/systems/global_systems.rs)
- [Procedural macros](../proto_ecs/ecs_macros/src)
