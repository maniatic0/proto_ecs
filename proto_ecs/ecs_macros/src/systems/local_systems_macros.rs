use syn::{
    self, token, parse_macro_input,
};
use quote::quote;
use crate::utils::{self, to_snake_case};
use crate::core_macros::ids;
use crate::systems::common::*;

struct LocalSystemArgs {
    struct_id: syn::Ident,
    dependencies: Dependencies,
    stages: Stages,
    before: DependencyList,
    after: DependencyList
}

impl syn::parse::Parse for LocalSystemArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let struct_id = input.parse::<syn::Ident>()?;
        let _ = input.parse::<token::Comma>()?;
        let mut dependencies: Option<Dependencies> = None;
        let mut stages: Option<Stages> = None;
        let mut before: Option<DependencyList> = None;
        let mut after: Option<DependencyList> = None;

        // Use this loop to parse a list of keyword arguments:
        // A = ...,
        // B = ...,
        loop {
            let keyword_arg = input.parse::<syn::Ident>();

                        


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
                            "Duplicated keyword argument: stages",
                        ));
                    }

                    before = Some(input.parse::<DependencyList>()?);
                },
                "after" => {
                    if after.is_some() {
                        return Err(syn::Error::new(
                            keyword_arg.span(),
                            "Duplicated keyword argument: stages",
                        ));
                    }

                    after = Some(input.parse::<DependencyList>()?);
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

        Ok(LocalSystemArgs {
            struct_id,
            dependencies: dependencies.unwrap_or(Dependencies(vec![])),
            stages: stages.unwrap_or(Stages(vec![])),
            before: before.unwrap_or(DependencyList(vec![])),
            after: after.unwrap_or(DependencyList(vec![])),
        })
    }
}

/// Create a new glue function to call user defined functions.
/// Return the ident of the new generated function and the function itself
/// as a token stream
fn create_glue_function(
    struct_id: &syn::Ident,
    function_id: &syn::Ident,
    args: &Vec<OptionalDep>,
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

    let arg_ids =
        (0..args.len()).map(
            |i| 
            syn::Ident::new(format!("arg{i}").as_str(), function_id.span())
        );

    // required to prevent use-after-move error later on this function
    let arg_ids_copy = arg_ids.clone();

    let arg_values = args.iter().enumerate().map(|(i, arg)| {
        let index = syn::Index::from(i);
        let type_id = arg.unwrap();
        let arg_value = quote! {
            (&mut *entity_datagroups_ptr.add(indices[#index] as usize))
            .as_any_mut()
            .downcast_mut::<#type_id>()
            .expect("Couldn't perform cast")
        };

        match arg {
            OptionalDep::OptionalDep(_) => {
                quote! {
                    if indices[#index] == proto_ecs::entities::entity::INVALID_DATAGROUP_INDEX
                    {
                        None
                    }
                    else
                    {
                        Some(#arg_value)
                    }
                }
            }
            OptionalDep::Dependency(_) => arg_value,
        }
    });

    let new_function = quote! {
        fn #new_function_id(world : &proto_ecs::entities::entity_system::World, entity : proto_ecs::entities::entity::EntityID, indices : &[proto_ecs::entities::entity::DataGroupIndexingType], entity_datagroups : &mut [std::boxed::Box<dyn proto_ecs::data_group::DataGroup>])
        {
            debug_assert!({
                let mut unique_set = std::collections::HashSet::new();
                indices.iter().all(|&i| {{unique_set.insert(i) && (i as usize) < entity_datagroups.len()}})
            }, "Overlapping indices or index out of range");

            unsafe {
                let entity_datagroups_ptr = entity_datagroups.as_mut_ptr();
                #(let #arg_ids = #arg_values;)*
                #struct_id :: #function_id (&world, entity, #( #arg_ids_copy, )*);
            }
        }
    };

    return (new_function_id, new_function);
}

pub fn register_local_system(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as LocalSystemArgs);
    let deps = args.dependencies.0;
    let struct_id_str = args.struct_id.to_string();
    let name_crc = crc32fast::hash(struct_id_str.as_bytes());
    let stages = args.stages.0;
    let new_trait_id = syn::Ident::new(
        format!("{}LocalSystem", struct_id_str).as_str(),
        args.struct_id.span(),
    );

    // Generate the simple spawn preparation for dependency datagroups
    let datagroups_simple_prepare: Vec<proc_macro2::TokenStream> = deps.iter().filter_map(|dep| {
        match dep {
            OptionalDep::OptionalDep(_) => None,
            OptionalDep::Dependency(d) => {
                let msg = format!("Local System '{}' added Datagroup dependency '{d}'", args.struct_id);

                Some(quote!{
                    proto_ecs::entities::entity_spawn_desc::helpers::local_system_try_add_datagroup::<#d>(spawn_desc, #msg);
                })
            },
        }
    }).collect();

    // Generate function arguments for trait functions
    let function_args = 
        {
            // Id of the entity holding this local system
            let mut args = vec![quote!(world : &proto_ecs::entities::entity_system::World, entity_id : proto_ecs::entities::entity::EntityID)];

            // Actual datagroup arguments
            args.extend(
                deps
                .iter()
                .map(|dep| {
                    let to_arg_name = |d: &syn::Ident| {
                        let ident_str = utils::to_snake_case(d.to_string().as_str());
                        return syn::Ident::new(ident_str.as_str(), d.span());
                    };

                    match dep {
                        OptionalDep::Dependency(d) => {
                            let arg_name = to_arg_name(d);
                            quote! { #arg_name : &mut #d }
                        }
                        OptionalDep::OptionalDep(d) => {
                            let arg_name = to_arg_name(d);
                            quote! { #arg_name : Option<&mut #d> }
                        }
                    }
                })
                .collect::<Vec<proc_macro2::TokenStream>>()
            );

            args
        };
    let function_ids = stages
        .iter()
        .map(|stage| {
            let stage_name = format!("stage_{}", stage.base10_digits());
            let function_id = syn::Ident::new(stage_name.as_str(), stage.span());
            function_id
        })
        .collect::<Vec<syn::Ident>>();

    let function_signatures = function_ids.iter().map(|ident| {
        quote! { fn #ident(#(#function_args),*) }
    });

    let glue_functions = function_ids
        .iter()
        .map(|function_id| create_glue_function(&args.struct_id, function_id, &deps));

    let glue_function_bodies = glue_functions.clone().map(|(_, body)| body);
    let glue_function_ids = glue_functions.map(|(id, _)| id);
    let stage_indices = stages
        .iter()
        .map(|lit| syn::Index::from(lit.base10_parse::<usize>().unwrap()));
    let struct_id = &args.struct_id;

    let mut result = quote!{};
    let id_magic_ident = ids::implement_id_traits(struct_id, &mut result);
    let before = args.before.0;
    let after = args.after.0;
    let id_set_up_fn_id = syn::Ident::new(
        format!("__{}_id_register__", to_snake_case(struct_id_str.as_str())).as_str(), 
        struct_id.span());

    result.extend(quote!{

        // For static assertions
        const _ : fn() = || {
            fn check_implements_traits<T : #new_trait_id>(){};
            check_implements_traits::<#struct_id>();
        };

        fn #id_set_up_fn_id (new_id : proto_ecs::systems::local_systems::SystemClassID)
        {
            #id_magic_ident.set(new_id).expect("Can't set id twice");
        }

        // Generate the trait to be implemented by the user 
        pub trait #new_trait_id 
        {
           #(#function_signatures;)*
        }

        #(#glue_function_bodies)*

        impl #struct_id
        {
            #[doc = "Simple preparation of this local system. Dependencies that require init args are left uninitialized. Dependencies with optional args are left empty"]
            pub fn simple_prepare(spawn_desc : &mut proto_ecs::entities::entity_spawn_desc::EntitySpawnDescription) -> bool
            {
                #(#datagroups_simple_prepare)*

                spawn_desc.add_local_system::<#struct_id>()
            }
        }

        impl proto_ecs::systems::local_systems::LocalSystemDesc for #struct_id 
        {
            #[doc = "Name of this local system"]
            const NAME : &'static str = #struct_id_str;
            #[doc = "Name's crc"]
            const NAME_CRC : u32 = #name_crc;
        }

        // Register this new local system to be loaded later
        const _ : () =
        {
            #[ctor::ctor]
            fn __register_local_system__()
            {
                proto_ecs::systems::local_systems::LocalSystemRegistry::register_lambda(
                    Box::new(
                        |registry| {
                            let mut dependencies = Vec::new();
                            let mut func_map  = proto_ecs::systems::local_systems::EMPTY_STAGE_MAP;
                            #( dependencies.push(#deps);)*
                            #( func_map[#stage_indices] = Some(#glue_function_ids);)*

                            assert!(
                                dependencies.len() <= proto_ecs::entities::entity::MAX_DATAGROUP_LEN as usize,
                                "Local System '{}' has more datagroups dependencies than what the indexing type can support: {} (limit {})",
                                #struct_id_str,
                                dependencies.len(),
                                proto_ecs::entities::entity::MAX_DATAGROUP_LEN
                            );

                            registry.register(
                                proto_ecs::systems::local_systems::LocalSystemRegistryEntry{
                                    id : proto_ecs::systems::local_systems::INVALID_SYSTEM_CLASS_ID,
                                    name : #struct_id_str,
                                    name_crc : #name_crc,
                                    dependencies : dependencies,
                                    functions : func_map,
                                    before : vec![
                                        #(<#before as proto_ecs::systems::local_systems::LocalSystemDesc>::NAME_CRC),*
                                    ],
                                    after : vec![
                                        #(<#after as proto_ecs::systems::local_systems::LocalSystemDesc>::NAME_CRC),*
                                    ],
                                    set_id_fn : #id_set_up_fn_id
                                }
                            );
                        }
                    )
                );
            }
        };
    });

    return result.into();
}
