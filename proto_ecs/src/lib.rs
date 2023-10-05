pub mod data_group;
pub mod entity;
pub mod local_systems;
pub mod core;

// Kinda hack so that we can export proc macros. 
extern crate self as proto_ecs;

#[cfg(test)]
mod tests;
