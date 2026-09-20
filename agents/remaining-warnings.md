# Remaining compiler warnings

After the recent cleanup, the remaining warnings are limited to the model asset manager and the layer iterator API. They do not currently prevent compilation or tests from passing.

## 1. Unused model lookup

Location: `proto_ecs/src/core/assets_management/models.rs:47`

Current pattern:

```rust
for handle in handles {
    let model = self.model_allocator.get(*handle);
    result.push(*handle);
}
```

The model is retrieved but never inspected. The function only returns the handles, so the lookup appears to be leftover code.

Recommended fix:

```rust
for handle in handles {
    result.push(*handle);
}
```

If the lookup was intended to validate that every cached handle is still live, make that intention explicit with a check and an appropriate error path. Otherwise, remove it.

## 2. Unused `unload_from_path` method

Location: `proto_ecs/src/core/assets_management/models.rs:128`

`ModelManager::unload_from_path` is private and currently has no callers. It frees model handles and removes the path from the loaded-model map, but the engine does not yet expose a model-unloading workflow.

There are two reasonable options:

### Option A: implement unloading

Add a public or controlled resource-management path such as:

```rust
Render::unload_model(path)
```

Before freeing the model, define the ownership rules for:

- CPU-side model data.
- GPU vertex/index buffers.
- Existing render proxies that still reference the model handle.
- Materials or entities using the model.

The render thread must not draw a model after its GPU resources have been released.

### Option B: remove or temporarily annotate it

If unloading is not part of the current scope, remove the method until the ownership design is ready. A temporary `#[allow(dead_code)]` is possible, but removing unused code is clearer.

## 3. Hidden lifetimes in layer iterators

Location: `proto_ecs/src/core/layer.rs:118-130`

Current signatures include:

```rust
pub fn layers_iter(&self) -> Iter<LayerContainer>
pub fn layers_iter_mut(&mut self) -> IterMut<LayerContainer>
```

The iterator borrows from `self`, but the returned lifetime is hidden. Rust accepts this, but recommends making it explicit:

```rust
pub fn layers_iter(&self) -> Iter<'_, LayerContainer>
pub fn layers_iter_mut(&mut self) -> IterMut<'_, LayerContainer>
pub fn overlays_iter(&self) -> Iter<'_, LayerContainer>
pub fn overlays_iter_mut(&mut self) -> IterMut<'_, LayerContainer>
```

This is a documentation/readability improvement, not a behavior change.

While changing these signatures, consider changing `overlays_iter` from `&mut self` to `&self`, since an immutable iterator does not need mutable access. That is a small API improvement but should be checked against all callers.

## Suggested order

1. Fix the unused model lookup; it is a straightforward removal.
2. Add explicit iterator lifetimes and review the unnecessary mutable receiver.
3. Decide whether model unloading belongs in the current renderer milestone.
4. If unloading is implemented, add tests for model-handle ownership and render-thread safety before exposing it publicly.

## Verification after future changes

```powershell
cargo test --manifest-path proto_ecs\Cargo.toml
cargo check --manifest-path sandbox\Cargo.toml
```

The desired result is a clean check, or only warnings that are deliberately documented and justified.
