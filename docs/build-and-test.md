# Build and test status

## Verified commands

The following commands were run successfully:

```powershell
cargo check --manifest-path proto_ecs\Cargo.toml
cargo check --manifest-path sandbox\Cargo.toml
cargo test --manifest-path proto_ecs\Cargo.toml
```

The library test result was:

```text
29 passed, 0 failed
```

The sandbox package, including `example_render`, also passed `cargo check`.

## What the tests cover

- Datagroup registration and initialization.
- Local/global system registration and ordering.
- Entity allocation, reuse, and stale-pointer protection.
- Entity creation and deletion.
- Parenting and transform hierarchy updates.
- Global-system lifetime behavior.
- Expected failures for invalid dependencies and spawn descriptions.

## Current warnings

Compilation succeeds with warnings, including:

- An unused model variable and currently unused model-unloading method.
- Elided lifetime syntax in layer iterator return types.
- Function-pointer comparisons in datagroup tests.
- An unused entity ID in the model-rendering sandbox example.

These do not currently prevent compilation or tests from passing.

## Current limitations visible in the code

- The root directory has no Cargo workspace manifest.
- The platform implementation is currently Windows-specific.
- The render thread uses busy-waiting while waiting for frame data.
- Some cleanup and resource-lifetime paths are still marked as TODO.
- The graphical examples were compiled but not launched during documentation work.
