mod data_group;
mod data_groups2;
mod entity;
mod systems;
mod core;

// Kinda hack so that we can export proc macros. 
extern crate self as proto_ecs;

#[cfg(test)]
mod tests;
