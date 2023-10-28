use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use proto_ecs::{
    app::App,
    entities::{
        entity_spawn_desc::EntitySpawnDescription,
        entity_system::{EntitySystem, DEFAULT_WORLD},
    },
};

pub mod shared_datagroups;

use shared_datagroups::{TestNumberDataGroup, TestNumberDataGroupArg};

// Move this once modules work

use ecs_macros::register_local_system;
use proto_ecs::entities::{entity::EntityID, entity_system::World};

pub struct TestAdder;

register_local_system! {
    TestAdder,
    dependencies = (TestNumberDataGroup),
    stages = (0)
}

impl TestAdderLocalSystem for TestAdder {
    fn stage_0(
        _world: &World,
        _entity_id: EntityID,
        test_number_data_group: &mut TestNumberDataGroup,
    ) {
        test_number_data_group.num = test_number_data_group.num + 1
    }
}

pub struct TestMultiplier;

register_local_system! {
    TestMultiplier,
    dependencies = (TestNumberDataGroup),
    stages = (0),
    after = (TestAdder)
}

impl TestMultiplierLocalSystem for TestMultiplier {
    fn stage_0(
        _world: &World,
        _entity_id: EntityID,
        test_number_data_group: &mut TestNumberDataGroup,
    ) {
        test_number_data_group.num = test_number_data_group.num * 2
    }
}

// Move this once modules work

fn entity_system_creation_benchmark(c: &mut Criterion) {
    if !App::is_initialized() {
        App::initialize();
    }

    let es = EntitySystem::get();
    es.reset(); // In case other tests happened
    es.step(0.0, 0.0); // Process reset

    c.bench_function("Entity System: Entity Creation", |b| {
        b.iter(|| {
            let mut spawn_desc = EntitySpawnDescription::default();
            let init_params = Box::new(TestNumberDataGroupArg { num: 1 });

            TestNumberDataGroup::prepare_spawn(&mut spawn_desc, init_params);
            TestAdder::simple_prepare(&mut spawn_desc);
            TestMultiplier::simple_prepare(&mut spawn_desc);
            spawn_desc.check_local_systems_panic();

            spawn_desc.set_name("Test Name".to_owned());

            es.create_entity(DEFAULT_WORLD, spawn_desc)
                .expect("Failed to create entity!");
        });
    });
}

fn entity_system_step_100_benchmark(c: &mut Criterion) {
    if !App::is_initialized() {
        App::initialize();
    }

    let es = EntitySystem::get();
    es.reset(); // In case other tests happened
    es.step(0.0, 0.0); // Process reset

    const ENTITIES_NUM: usize = 100;

    for _ in 0..ENTITIES_NUM {
        let mut spawn_desc = EntitySpawnDescription::default();
        let init_params = Box::new(TestNumberDataGroupArg { num: 1 });

        TestNumberDataGroup::prepare_spawn(&mut spawn_desc, init_params);
        TestAdder::simple_prepare(&mut spawn_desc);
        TestMultiplier::simple_prepare(&mut spawn_desc);
        spawn_desc.check_local_systems_panic();

        spawn_desc.set_name("Test Name".to_owned());

        es.create_entity(DEFAULT_WORLD, spawn_desc)
            .expect("Failed to create entity!");
    }

    let mut group = c.benchmark_group("entity-system-throughput-100");
    group.throughput(Throughput::Elements(ENTITIES_NUM as u64));
    group.bench_function("Entity System: Step 100", |b| {
        b.iter(|| {
            es.step(0.0, 0.0);
        });
    });
}

fn entity_system_step_10k_benchmark(c: &mut Criterion) {
    if !App::is_initialized() {
        App::initialize();
    }

    let es = EntitySystem::get();
    es.reset(); // In case other tests happened
    es.step(0.0, 0.0); // Process reset

    const ENTITIES_NUM: usize = 10_000;

    for _ in 0..ENTITIES_NUM {
        let mut spawn_desc = EntitySpawnDescription::default();
        let init_params = Box::new(TestNumberDataGroupArg { num: 1 });

        TestNumberDataGroup::prepare_spawn(&mut spawn_desc, init_params);
        TestAdder::simple_prepare(&mut spawn_desc);
        TestMultiplier::simple_prepare(&mut spawn_desc);
        spawn_desc.check_local_systems_panic();

        spawn_desc.set_name("Test Name".to_owned());

        es.create_entity(DEFAULT_WORLD, spawn_desc)
            .expect("Failed to create entity!");
    }

    let mut group = c.benchmark_group("entity-system-throughput-10k");
    group.throughput(Throughput::Elements(ENTITIES_NUM as u64));
    group.bench_function("Entity System: Step 10k", |b| {
        b.iter(|| {
            es.step(0.0, 0.0);
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(50);
    targets = entity_system_creation_benchmark, entity_system_step_100_benchmark, entity_system_step_10k_benchmark
);
criterion_main!(benches);
