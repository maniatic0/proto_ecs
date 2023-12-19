use crate::core_macros::ids;
use crc32fast;
use proc_macro;
use quote::quote;
use syn::{self, parse::Parse, parse_macro_input};
use crate::common::*;

// -- < Datagroups > -----------------------------------

#[inline]
fn get_datagroup_desc_trait(datagroup: &syn::Ident) -> syn::Ident {
    let datagroup_str = datagroup.to_string();
    syn::Ident::new(&format!("{datagroup_str}Desc"), datagroup.span())
}

/// Register the way a datagroup struct initializes
fn register_datagroup_init(args: &DatagroupInput, result : &mut proc_macro2::TokenStream) {
    let datagroup_desc_trait = get_datagroup_desc_trait(&args.datagroup);

    let init_fn_trait = args.init_style.to_signature();

    let init_fn_arg_trait_check = match &args.init_style {
        InitArgStyle::NoInit => quote! {},
        InitArgStyle::NoArg => quote! {},
        InitArgStyle::Arg(arg) => {
            let arg_clone = arg.clone();
            quote!(
                const _: fn() = || {
                    /// Only callable when Arg implements trait GenericDataGroupInitArgTrait.
                    fn check_cast_trait_implemented<T: ?Sized + proto_ecs::data_group::GenericDataGroupInitArgTrait>() {}
                    check_cast_trait_implemented::<#arg_clone>();
                    // Based on https://docs.rs/static_assertions/latest/static_assertions/macro.assert_impl_all.html
                };
            )
        }
        InitArgStyle::OptionalArg(arg) => {
            let arg_clone = arg.clone();
            quote!(
                const _: fn() = || {
                    /// Only callable when Arg implements trait GenericDataGroupInitArgTrait.
                    fn check_cast_trait_implemented<T: ?Sized + proto_ecs::data_group::GenericDataGroupInitArgTrait>() {}
                    check_cast_trait_implemented::<#arg_clone>();
                    // Based on https://docs.rs/static_assertions/latest/static_assertions/macro.assert_impl_all.html
                };
            )
        }
    };

    let init_fn_internal = match &args.init_style {
        InitArgStyle::NoInit => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::data_group::GenericDataGroupInitArg>)
            {
                panic!("Datagroup with no init!");
            }
        },
        InitArgStyle::NoArg => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::data_group::GenericDataGroupInitArg>)
            {
                assert!(_init_data.is_none(), "Unexpected init data!");
                self.init();
            }
        },
        InitArgStyle::Arg(_) => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::data_group::GenericDataGroupInitArg>)
            {
                let _init_data = _init_data.expect("Missing init data!");
                let _init_data = proto_ecs::core::casting::into_any(_init_data);
                self.init(_init_data);
            }
        },
        InitArgStyle::OptionalArg(_) => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::data_group::GenericDataGroupInitArg>)
            {
                let _init_data = _init_data.and_then(|v| Some(proto_ecs::core::casting::into_any(v)));
                self.init(_init_data);
            }
        },
    };

    let init_arg_type_desc = args.init_style.to_type_param();

    let init_const_desc = args.init_style.to_init_const_desc();

    let datagroup = &args.datagroup;
    let prepare_fn = match &args.init_style {
        InitArgStyle::NoInit => {
            let msg = format!(
                "Add data group {} to an entity being prepared to spawn",
                datagroup
            );

            quote!(
                #[doc = #msg]
                pub fn prepare_spawn(spawn_desc : &mut proto_ecs::entities::entity_spawn_desc::EntitySpawnDescription) -> std::option::Option<proto_ecs::data_group::DataGroupInitType> {
                    spawn_desc.add_datagroup::<#datagroup>(proto_ecs::data_group::DataGroupInitType::NoInit)
                }
            )
        }
        InitArgStyle::NoArg => {
            let msg = format!(
                "Add data group {} to an entity being prepared to spawn. It will init",
                datagroup
            );

            quote!(
                #[doc = #msg]
                pub fn prepare_spawn(spawn_desc : &mut proto_ecs::entities::entity_spawn_desc::EntitySpawnDescription) -> std::option::Option<proto_ecs::data_group::DataGroupInitType> {
                    spawn_desc.add_datagroup::<#datagroup>(proto_ecs::data_group::DataGroupInitType::NoArg)
                }
            )
        }
        InitArgStyle::Arg(arg) => {
            let msg = format!(
                "Add data group {} to an entity being prepared to spawn. It will init with arg {}",
                datagroup, arg
            );

            quote!(
                #[doc = #msg]
                pub fn prepare_spawn(spawn_desc : &mut proto_ecs::entities::entity_spawn_desc::EntitySpawnDescription, arg : std::boxed::Box<#arg>) -> std::option::Option<proto_ecs::data_group::DataGroupInitType> {
                    spawn_desc.add_datagroup::<#datagroup>(proto_ecs::data_group::DataGroupInitType::Arg(arg))
                }
            )
        }
        InitArgStyle::OptionalArg(arg) => {
            let msg = format!("Add data group {} to an entity being prepared to spawn. It will init with optional arg {}", datagroup, arg);

            quote!(
                #[doc = #msg]
                pub fn prepare_spawn(spawn_desc : &mut proto_ecs::entities::entity_spawn_desc::EntitySpawnDescription, arg : std::option::Option<std::boxed::Box<#arg>>) -> std::option::Option<proto_ecs::data_group::DataGroupInitType> {
                    spawn_desc.add_datagroup::<#datagroup>(proto_ecs::data_group::DataGroupInitType::OptionalArg(arg))
                }
            )
        }
    };

    result.extend(quote! {
        #init_fn_arg_trait_check
        trait #datagroup_desc_trait {
            #init_fn_trait
        }

        impl proto_ecs::data_group::DataGroup for #datagroup
        {
            #init_fn_internal
        }

        impl proto_ecs::data_group::DataGroupInitDescTrait for #datagroup
        {
            #[doc = "Arg type, if any"]
            #init_arg_type_desc

            #[doc = "Init Description of this DataGroup"]
            #init_const_desc
        }

        impl #datagroup
        {
            #prepare_fn
        }
    });


}

#[derive(Clone)]
struct DatagroupInput {
    datagroup: syn::Ident,
    factory: syn::Ident,
    init_style: InitArgStyle,
}

impl Parse for DatagroupInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let datagroup = input.parse::<syn::Ident>().or_else(|_| 
        {
            return Err(syn::Error::new(input.span(), "Missing Datagroup Struct Identifier"));
        })?;

        let _ = input.parse::<syn::token::Comma>()?;

        let factory = input.parse::<syn::Ident>().or_else(|_|
        {
            return Err(syn::Error::new(input.span(), "Missing factory function argument"));
        })?;

        let _ = input.parse::<syn::token::Comma>()?;

        let mut init_style = None;

        loop {
            let keyword_arg = input.parse::<syn::Ident>();
            // Return if already parsed all keyword arguments
            let keyword_arg = match keyword_arg {
                Err(_) => break,
                Ok(val) => val
            };

            let keyword_arg_str = keyword_arg.to_string();
            let _ = input.parse::<syn::Token![=]>();

            match keyword_arg_str.as_str()
            {
                "init_style" => {

                    if init_style.is_some()
                    {
                        return Err(syn::Error::new(
                            keyword_arg.span(),
                            "Duplicated keyword argument: init_style",
                        ));
                    }

                    init_style = Some(input.parse::<InitArgStyle>()?);
                },

                _ => {
                    return Err(syn::Error::new(
                        keyword_arg.span(),
                        "Unexpected keyword. Available keywords = {init_style}")
                    )
                }
            }
        }

        return Ok(
            DatagroupInput { 
                datagroup, factory, 
                init_style: init_style.unwrap_or(InitArgStyle::NoInit) 
            });
    }
}

/// Register a datagroup struct as a new datagroup class in the global registry
pub fn register_datagroup(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as DatagroupInput);
    let DatagroupInput { datagroup, factory , ..} = args.clone();
    let datagroup_str = datagroup.to_string();
    let name_crc = crc32fast::hash(datagroup_str.as_bytes());
    let datagroup_desc_trait = get_datagroup_desc_trait(&datagroup);

    let mut result = quote!();
    let datagroup_id_magic_ident = ids::implement_id_traits(&datagroup, &mut result);

    result.extend(quote! {

        const _: fn() = || {
            /// Only callable when Datagroup implements trait DatagroupDesc.
            fn check_desc_trait_implemented<T: ?Sized + #datagroup_desc_trait>() {}
            check_desc_trait_implemented::<#datagroup>();
            // Based on https://docs.rs/static_assertions/latest/static_assertions/macro.assert_impl_all.html
        };

        impl proto_ecs::data_group::DatagroupDesc for #datagroup
        {
            #[doc = "Name of this datagroup"]
            const NAME : &'static str = #datagroup_str;
            #[doc = "Name's crc"]
            const NAME_CRC : u32 = #name_crc;
            #[doc = "Factory to create new instances of this datagroup"]
            const FACTORY : proto_ecs::data_group::DataGroupFactory = #factory;
        }

        // Registration in the global datagroup registry
        const _ : () = {
            #[ctor::ctor]
            fn __register_datagroup__()
            {
                proto_ecs::data_group::DataGroupRegistry::register_lambda(
                    std::boxed::Box::new(
                        |registry| {
                            let new_id = registry.register(proto_ecs::data_group::DataGroupRegistryEntry{
                                name: <#datagroup as proto_ecs::data_group::DatagroupDesc>::NAME,
                                name_crc: <#datagroup as proto_ecs::data_group::DatagroupDesc>::NAME_CRC,
                                factory_func: <#datagroup as proto_ecs::data_group::DatagroupDesc>::FACTORY,
                                init_desc: <#datagroup as proto_ecs::data_group::DataGroupInitDescTrait>::INIT_DESC,
                                id: proto_ecs::data_group::DataGroupID::MAX
                            });
                            #datagroup_id_magic_ident.set(new_id).expect("Failed to register DataGroup ID");
                        }
                    )
                );
            }
        };
    });

    register_datagroup_init(&args, &mut result);
    return result.into();
}
