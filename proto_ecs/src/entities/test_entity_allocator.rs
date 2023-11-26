use proto_ecs::entities::entity_allocator::*;
use crate::entities::entity_spawn_desc::EntitySpawnDescription;
use crate::app::App;

#[test]
fn test_allocation()
{
    if !App::is_initialized() {
        App::initialize();
    }

    let mut alloc = EntityAllocator::new();
    let mut entity_ptr = alloc.allocate();
    let mut spawn_desc = EntitySpawnDescription::default();
    spawn_desc.set_name("hello".to_owned());

    // Check that the pointer says entity is live
    assert!(entity_ptr.is_live());

    // Check that entity initialized state is consistent
    assert!(!entity_ptr.is_initialized());
    entity_ptr.init(420, spawn_desc);
    assert!(entity_ptr.is_initialized());

    // Check that we can access to the entity without segfaulting and initialization went ok
    assert_eq!(entity_ptr.read().get_name(), "hello".to_owned());
    assert_eq!(entity_ptr.read().get_id(), 420);
}

#[test]
fn test_free()
{
    if !App::is_initialized() {
        App::initialize();
    }

    let mut alloc = EntityAllocator::new();
    let entity_ptr = alloc.allocate();

    // Check that you can free without initializing 
    alloc.free(&entity_ptr);
    assert!(!entity_ptr.is_live());

    // Check that we can free the entity after init and the initialization state is consistent
    let mut entity_ptr = alloc.allocate();
    let spawn_desc = EntitySpawnDescription::default();
    entity_ptr.init(420, spawn_desc);

    assert!(entity_ptr.is_initialized());
    alloc.free(&entity_ptr);
    assert!(!entity_ptr.is_initialized());
    assert!(!entity_ptr.is_live());
}

#[test]
#[should_panic]
fn test_panic_use_after_free()
{
    if !App::is_initialized() {
        App::initialize();
    }
    
    let mut alloc = EntityAllocator::new();
    let mut entity_ptr = alloc.allocate();
    let spawn_desc = EntitySpawnDescription::default();

    entity_ptr.init(420, spawn_desc);
    alloc.free(&entity_ptr);
    entity_ptr.read().get_id();
}