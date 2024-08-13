pub mod app;
pub mod core;
pub mod data_group;
pub mod entities;
pub mod prelude;
pub mod systems;

// Kinda hack so that we can export proc macros.
extern crate self as proto_ecs;

#[cfg(test)]
mod tests;
