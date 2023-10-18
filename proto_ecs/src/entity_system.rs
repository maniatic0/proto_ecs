use crate::entity::EntityID;

/// We just go up. If we ever run out of them we can think of blocks of IDs per thread and a better allocation system
static ENTITY_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Allocate a new Entity ID
pub fn allocate_entity_id() -> EntityID {
    ENTITY_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Deallocate an Entity ID
pub fn deallocate_entity_id(id: EntityID) {
    assert!(id < ENTITY_COUNT.load(std::sync::atomic::Ordering::Relaxed));

    // Note: if we ever need to do something more complex with IDs we can do it here
}