use crate::common::*;
use crate::core_macros::ids::implement_id_traits;
use crate::systems::common::*;
use proc_macro;
use quote::quote;

/// Arguments required to declare a GlobalSystem.
///
/// * `struct_id` : Name of a struct being declared as global system
/// * `dependencies` : A list of datagroups that are assumed to be dependencies of this system
/// * `stages` : A list of numbers identifying the stages that this global system should run on
/// * `before` : A list of other global systems that should run after this system (this global system runs BEFORE ...)
/// * `after` : A list of other global systems that should run before this system (this global system runs AFTER ...)
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

    let trait_name = format!("{}GlobalSystem", struct_id.to_string());
    let global_system_trait = syn::Ident::new(&trait_name, struct_id.span());
    let trait_function_ids =
    {
        let active_stages = match stages.to_ints()
        {
            Ok(is) => is,
            Err(e) => {
                return e.into_compile_error().into();
            }
        };

        active_stages
            .into_iter()
            .map(
                |i| 
                syn::Ident::new(
                    format!("stage_{i}").as_str(), 
                    struct_id.span()
                )
            )
    }; 

    let trait_function_signatures = trait_function_ids.map(|id| {
        quote!(fn #id(&mut self, entity_map : proto_ecs::systems::global_systems::EntityMap);)
    });

    let init_fn_signature = init_style.to_signature();
    let mut result = quote!();

    // TODO Implement code that will use this variable to set the valid id
    let id_variable = implement_id_traits(&struct_id, &mut result);

    result.extend(quote! {
        // We create a trait implementing all the mandatory functions for this global system
        pub trait #global_system_trait
        {
            #(#trait_function_signatures)*

            #init_fn_signature
        }

        // Now we auto implement the global system trait 
        impl proto_ecs::systems::global_systems::GlobalSystem for #struct_id
        {
            fn __init__(&mut self)
            {
                // TODO implement this function
            }
        }


    });


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