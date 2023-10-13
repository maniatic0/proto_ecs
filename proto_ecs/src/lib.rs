pub mod app;
pub mod core;
pub mod data_group;
pub mod entity;
pub mod local_systems;
pub mod entity_spawn_desc;

// Kinda hack so that we can export proc macros.
extern crate self as proto_ecs;

#[cfg(test)]
mod tests;
