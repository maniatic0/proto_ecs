pub mod data_group;
pub mod data_group2;
pub mod entity;
pub mod systems;
pub mod core;

// Kinda hack so that we can export proc macros. 
extern crate self as proto_ecs;

#[cfg(test)]
mod tests;
