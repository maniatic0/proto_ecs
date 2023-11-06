#[cfg(test)]
mod global_system_test{
    use crate::get_id;
    use crate::tests::shared_global_systems::sgs::{Test, TestAfter, TestBefore};
    use crate::systems::global_systems::{GlobalSystemRegistry, EntityMap};
    use crate::app::App;
    use crate::core::casting::cast_mut;

    #[test]
    fn test_global_system_registration()
    {
        if !App::is_initialized()
        {
            App::initialize();
        }
        let gs_registry = GlobalSystemRegistry
                                                                ::get_global_registry()
                                                                .read();

        let test_entry = gs_registry.get_entry::<Test>();
        let before_entry = gs_registry.get_entry::<TestBefore>();
        let after_entry = gs_registry.get_entry::<TestAfter>();
        
        assert_eq!(test_entry.id, get_id!(Test));
        assert_eq!(before_entry.id, get_id!(TestBefore));
        assert_eq!(after_entry.id, get_id!(TestAfter));

        for (i, f) in test_entry.functions.iter().enumerate()
        {
            assert!( (i != 42 && f.is_none()) || (i == 42 && f.is_some()), "Missing registered function");
        }

        assert!(get_id!(Test) > get_id!(TestBefore), "Toposort error: Test GS should run before TestBefore");
        assert!(get_id!(Test) < get_id!(TestAfter), "Toposort error: Test GS should run after TestAfter");
    }

    #[test]
    fn test_global_system_initialization()
    {
        if !App::is_initialized()
        {
            App::initialize();
        }

        let gs_registry = GlobalSystemRegistry
                                                                ::get_global_registry()
                                                                .read();

        {
            // Test that state remains the same when initializing without args
            let mut test_gs = gs_registry.create::<Test>();
            test_gs.__init__(None);
            let test_gs : &mut Test = cast_mut(&mut test_gs);
            assert_eq!(test_gs._a, 69);
            assert_eq!(test_gs._b, "Hello world".to_string());  
        }

        {
            let mut test_gs = gs_registry.create::<Test>();
            test_gs.__init__(Some(Box::new(Test{_a: 42, _b: "foo".to_string()})));
            let test_gs : &mut Test = cast_mut(&mut test_gs);
            assert_eq!(test_gs._a, 42);
            assert_eq!(test_gs._b, "foo".to_string());
        }
    }

    #[test]
    fn test_global_system_run()
    {
        if !App::is_initialized()
        {
            App::initialize();
        }

        let gs_registry = GlobalSystemRegistry
                                                                ::get_global_registry()
                                                                .read();

        let mut test_gs = gs_registry.create::<Test>();
        let test_gs_entry = gs_registry.get_entry::<Test>();
        let entity_map = EntityMap::new();

        for f in test_gs_entry.functions
        {
            match f {
                Some(f) => (f)(&mut test_gs, &entity_map),
                _ => {}
            }
        }

        let test_gs: &mut Test = cast_mut(&mut test_gs);
        assert_eq!(test_gs._a, 69 * 2);
    }
}