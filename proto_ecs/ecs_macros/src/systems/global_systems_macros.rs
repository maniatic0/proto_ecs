use crate::common::*;
use crate::utils::to_snake_case;
use crate::core_macros::ids::implement_id_traits;
use crate::systems::common::*;
use proc_macro;
use quote::quote;

/// Arguments required to declare a GlobalSystem. 
/// * `struct_id` : Name of a struct being declared as global system
/// * `dependencies` : A list of datagroups that are assumed to be dependencies of this system
/// * `stages` : A list of numbers identifying the stages that this global system should run on
/// * `before` : A list of other global systems that should run after this system (this global system runs BEFORE ...)
/// * `after` : A list of other global systems that should run before this system (this global system runs AFTER...)
/// * `factory` : a function that takes no input and returns a Box<dyn GlobalSystem> returning an instance of this GlobalSystem
/// * `init_style` : The style of the input argument for the initialization functions. Optional? Required? None?
struct GlobalSystemArgs {
    struct_id: syn::Ident,
    dependencies: Dependencies,
    stages: Stages,
    before: DependencyList,
    after: DependencyList,
    factory: syn::Ident,
    init_style: InitArgStyle,
}

pub fn register_global_system(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let GlobalSystemArgs {
        struct_id,
        dependencies,
        stages,
        before,
        after,
        factory,
        init_style,
    } = syn::parse_macro_input!(args as GlobalSystemArgs);
    let before = before.0;
    let after = after.0;
    let deps = dependencies.0;
    let trait_name = format!("{}GlobalSystem", struct_id.to_string());
    let global_system_trait = syn::Ident::new(&trait_name, struct_id.span());
    let (trait_function_ids, stage_indices) =
    {
        let active_stages = match stages.to_ints()
        {
            Ok(is) => is,
            Err(e) => {
                return e.into_compile_error().into();
            }
        };
        let active_stages_clone = active_stages.clone();
        (active_stages_clone
            .into_iter()
            .map(
                |i| 
                syn::Ident::new(
                    format!("stage_{i}").as_str(), 
                    struct_id.span()
                )
            ),
        active_stages.clone()
            .into_iter()
            .map( 
                |i| 
                syn::Index::from(i as usize)
            )
        )
    }; 
    let struct_id_str = struct_id.to_string();
    let name_crc = crc32fast::hash(struct_id_str.as_bytes());
    let trait_function_signatures = trait_function_ids.clone().map(|id| {
        quote!(fn #id(&mut self, entity_map : proto_ecs::systems::global_systems::EntityMap);)
    });

    let init_fn_signature = init_style.to_signature();
    let mut result = quote!();

    let id_variable = implement_id_traits(&struct_id, &mut result);
    let glue_functions = trait_function_ids.clone().map(
        |function_id| create_glue_function(&struct_id, &function_id)
    );

    let glue_function_ids = glue_functions.clone().map(|(s,_)| s);
    let glue_function_bodies = glue_functions.map(|(_,b)| b);

    let global_system_desc_trait = syn::Ident::new(
        format!("{struct_id_str}Desc").as_str(),
        struct_id.span()
    );

    let init_fn_trait = init_style.to_signature();

    let init_fn_internal = match &init_style {
        InitArgStyle::NoInit => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::systems::global_systems::GenericGlobalSystemInitArg>)
            {
                panic!("Global System with no init!");
            }
        },
        InitArgStyle::NoArg => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::systems::global_systems::GenericGlobalSystemInitArg>)
            {
                assert!(_init_data.is_none(), "Unexpected init data!");
                self.init();
            }
        },
        InitArgStyle::Arg(_) => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::systems::global_systems::GenericGlobalSystemInitArg>)
            {
                let _init_data = _init_data.expect("Missing init data!");
                let _init_data = proto_ecs::core::casting::into_any(_init_data);
                self.init(_init_data);
            }
        },
        InitArgStyle::OptionalArg(_) => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::systems::global_systems::GenericGlobalSystemInitArg>)
            {
                let _init_data = _init_data.and_then(|v| Some(proto_ecs::core::casting::into_any(v)));
                self.init(_init_data);
            }
        },
    };

    let init_fn_arg_trait_check = match &init_style {
        InitArgStyle::NoInit => quote! {},
        InitArgStyle::NoArg => quote! {},
        InitArgStyle::Arg(arg) => {
            let arg_clone = arg.clone();
            quote!(
                const _: fn() = || {
                    /// Only callable when Arg implements trait GenericGlobalSystemInitArgTrait.
                    fn check_cast_trait_implemented<T: ?Sized + proto_ecs::systems::global_systems::GenericGlobalSystemInitArgTrait>() {}
                    check_cast_trait_implemented::<#arg_clone>();
                    // Based on https://docs.rs/static_assertions/latest/static_assertions/macro.assert_impl_all.html
                };
            )
        }
        InitArgStyle::OptionalArg(arg) => {
            let arg_clone = arg.clone();
            quote!(
                const _: fn() = || {
                    /// Only callable when Arg implements trait GenericGlobalSystemInitArgTrait.
                    fn check_cast_trait_implemented<T: ?Sized + proto_ecs::systems::global_systems::GenericGlobalSystemInitArgTrait>() {}
                    check_cast_trait_implemented::<#arg_clone>();
                    // Based on https://docs.rs/static_assertions/latest/static_assertions/macro.assert_impl_all.html
                };
            )
        }
    };

    let init_arg_type_desc = init_style.to_type_param();
    let init_const_desc = init_style.to_init_const_desc();

    result.extend(quote! {
        // Init arguments description
        #init_fn_arg_trait_check
        trait #global_system_desc_trait {
            #init_fn_trait
        }

        // We create a trait implementing all the mandatory functions for this global system
        pub trait #global_system_trait
        {
            #(#trait_function_signatures)*

            #init_fn_signature
        }

        #(#glue_function_bodies)*

        // Now we auto implement the global system trait 
        impl proto_ecs::systems::global_systems::GlobalSystem for #struct_id
        {
            #init_fn_internal
        }

        impl proto_ecs::systems::global_systems::GlobalSystemInitDescTrait for #struct_id
        {
            #[doc = "Arg type, if any"]
            #init_arg_type_desc

            #[doc = "Init Description of this global system"]
            #init_const_desc
        }

        // Register this global system into the global registry
        const _ : () = 
        {
            fn __set_global_system_id__(new_id : proto_ecs::systems::global_systems::GlobalSystemID)
            {
                #id_variable.set(new_id).expect("Can't set id twice");
            }
            #[ctor::ctor]
            fn __register_global_system__()
            {
                proto_ecs::systems::global_systems::GlobalSystemRegistry::register_lambda(
                    Box::new(
                        |registry| {
                            let mut dependencies = Vec::new();
                            #( dependencies.push(#deps);)*

                            let mut func_map  = proto_ecs::systems::global_systems::EMPTY_STAGE_MAP;

                            #( func_map[#stage_indices] = Some(#glue_function_ids);)*
                            
                            assert!(
                                dependencies.len() <= proto_ecs::entities::entity::MAX_DATAGROUP_INDEX as usize,
                                "Local System '{}' has more datagroups dependencies than what the indexing type can support: {} (limit {})",
                                #struct_id_str,
                                dependencies.len(),
                                proto_ecs::entities::entity::MAX_DATAGROUP_INDEX
                            );

                            registry.register(
                                proto_ecs::systems::global_systems::GlobalSystemRegistryEntry{
                                    id : proto_ecs::systems::global_systems::INVALID_GLOBAL_SYSTEM_CLASS_ID,
                                    name : #struct_id_str,
                                    name_crc : #name_crc,
                                    dependencies : dependencies,
                                    functions : func_map,
                                    before : vec![
                                        #(<#before as proto_ecs::systems::global_systems::GlobalSystemDesc>::NAME_CRC),*
                                    ],
                                    after : vec![
                                        #(<#after as proto_ecs::systems::global_systems::GlobalSystemDesc>::NAME_CRC),*
                                    ],
                                    factory : #factory,
                                    init_desc : <#struct_id as proto_ecs::systems::global_systems::GlobalSystemInitDescTrait>::INIT_DESC,
                                    set_id_fn : __set_global_system_id__
                                }
                            );
                        }
                    )
                );
            }
        };

    });

    // Add trait for init arguments if there's a struct for arguments
    match init_style 
    {
        InitArgStyle::Arg(id) | InitArgStyle::OptionalArg(id) => {
            result.extend(
                quote!{
                    impl proto_ecs::systems::global_systems::GenericGlobalSystemInitArgTrait for #id
                    { }
                }
            );
        }
        _ => {}
    }

    return result.into();
}

impl syn::parse::Parse for GlobalSystemArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let struct_id = input.parse::<syn::Ident>()?;
        let _ = input.parse::<syn::token::Comma>()?;
        let mut dependencies: Option<Dependencies> = None;
        let mut stages: Option<Stages> = None;
        let mut before: Option<DependencyList> = None;
        let mut after: Option<DependencyList> = None;
        let mut factory: Option<syn::Ident> = None;
        let mut init_style: Option<InitArgStyle> = None;

        // Use this loop to parse a list of keyword arguments:
        // A = ...,
        // B = ...,
        loop {
            let keyword_arg = input.parse::<syn::Ident>();

            // Return if already parsed all keyword arguments
            match keyword_arg {
                Err(_) => break,
                _ => {}
            };

            let _ = input.parse::<syn::Token![=]>();

            let keyword_arg = keyword_arg?;
            let keyword_arg_str = keyword_arg.to_string();
            match keyword_arg_str.as_str() {
                "dependencies" => {
                    if dependencies.is_some() {
                        return Err(syn::Error::new(
                            keyword_arg.span(),
                            "Duplicated keyword argument: dependencies",
                        ));
                    }
                    dependencies = Some(input.parse::<Dependencies>()?)
                }
                "stages" => {
                    if stages.is_some() {
                        return Err(syn::Error::new(
                            keyword_arg.span(),
                            "Duplicated keyword argument: stages",
                        ));
                    }

                    stages = Some(input.parse::<Stages>()?);
                }
                "before" => {
                    if before.is_some() {
                        return Err(syn::Error::new(
                            keyword_arg.span(),
                            "Duplicated keyword argument: before",
                        ));
                    }

                    before = Some(input.parse::<DependencyList>()?);
                }
                "after" => {
                    if after.is_some() {
                        return Err(syn::Error::new(
                            keyword_arg.span(),
                            "Duplicated keyword argument: after",
                        ));
                    }

                    after = Some(input.parse::<DependencyList>()?);
                }
                "init_arg" => {
                    if init_style.is_some() {
                        return Err(syn::Error::new(
                            keyword_arg.span(),
                            "Duplicated keyword argument: init_arg",
                        ));
                    }

                    init_style = Some(input.parse::<InitArgStyle>()?);
                }
                "factory" => {
                    if factory.is_some() {
                        return Err(syn::Error::new(
                            keyword_arg.span(),
                            "Duplicated keyword argument: factory",
                        ));
                    }
                    factory = Some(input.parse::<syn::Ident>()?);
                }
                _ => {
                    return Err(syn::Error::new(
                        keyword_arg.span(),
                        "Unexpected keyword. Available keywords = {dependencies, stages}",
                    ));
                }
            }

            let comma = input.parse::<syn::Token![,]>();
            if comma.is_err() {
                break;
            }
        }

        // Content should be ended by now
        if !input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "Unexpected token at the end of macro",
            ));
        }

        if factory.is_none() {
            return Err(syn::Error::new(
                input.span(),
                "Factory keyword argument is not optional, please provide a factory function.",
            ));
        }

        Ok(GlobalSystemArgs {
            struct_id,
            dependencies: dependencies.unwrap_or(Dependencies(vec![])),
            stages: stages.unwrap_or(Stages(vec![])),
            before: before.unwrap_or(DependencyList(vec![])),
            after: after.unwrap_or(DependencyList(vec![])),
            init_style: init_style.unwrap_or(InitArgStyle::NoInit),
            factory: factory.unwrap(),
        })
    }
}

fn create_glue_function(
    struct_id: &syn::Ident,
    function_id: &syn::Ident,
) -> (syn::Ident, proc_macro2::TokenStream) {
    let new_function_id = syn::Ident::new(
        format!(
            "_{}_{}_", 
                to_snake_case(
                    struct_id.to_string().as_str()
                ), 
                function_id.to_string()
            ).as_str(),
        function_id.span(),
    );

    let new_function = quote! {
        fn #new_function_id(
            global_system : &mut std::boxed::Box<dyn proto_ecs::systems::global_systems::GlobalSystem>, 
            entity_map : proto_ecs::systems::global_systems::EntityMap)
        {
            let mut global_system = global_system.as_any_mut().downcast_mut::<#struct_id>().unwrap();
            global_system. #function_id (entity_map);
        }
    };

    return (new_function_id, new_function);
}
