use crate::core_macros::ids;
use crc32fast;
use proc_macro;
use quote::quote;
use syn::{self, parenthesized, parse::Parse, parse_macro_input, token};

// -- < Datagroups > -----------------------------------

/// Whether a DataGroup has an init function
/// If it has one, it can specify if it doesn't take an argument,
/// if the argument is required, or if the argument is optional
enum DataGroupInit {
    /// No Init
    NoInit,
    /// Init with no Args
    NoArg,
    /// Init with Args
    Arg(syn::Ident),
    /// Init with optional Args
    OptionalArg(syn::Ident),
}

struct DataGroupInitParseDesc {
    pub datagroup_name: syn::Ident,
    pub init_type: DataGroupInit,
}

impl Parse for DataGroupInitParseDesc {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let datagroup_name: syn::Ident = input.parse()?;
        let _: token::Comma = input.parse()?; // comma token
        let init_type: syn::Ident = input.parse()?;

        match init_type.to_string().as_str() {
        "NoInit" => Ok(DataGroupInitParseDesc{ datagroup_name : datagroup_name.clone(), init_type : DataGroupInit::NoInit}),
        "NoArg" => Ok(DataGroupInitParseDesc{ datagroup_name : datagroup_name.clone(), init_type : DataGroupInit::NoArg}),
        "Arg" => {
            let content;
            let _ : token::Paren = parenthesized!(content in input); // parenthesis
            let arg_type : syn::Ident = content.parse()?;
            Ok(DataGroupInitParseDesc{ datagroup_name : datagroup_name.clone(), init_type : DataGroupInit::Arg(arg_type)})
        },
        "OptionalArg" => {
            let content;
            let _ : token::Paren = parenthesized!(content in input); // parenthesis
            let arg_type : syn::Ident = content.parse()?;
            Ok(DataGroupInitParseDesc{ datagroup_name : datagroup_name.clone(), init_type : DataGroupInit::OptionalArg(arg_type)})
        },
        unknown => Err(
            syn::Error::new(init_type.span(), format!("Unexpected Init type '{unknown}'. The only valids are: NoInit, NoArg, Arg(ArgType), OptionalArg(ArgType)"))
        )
       }
    }
}

#[inline]
fn get_datagroup_desc_trait(datagroup: &syn::Ident) -> syn::Ident {
    let datagroup_str = datagroup.to_string();
    syn::Ident::new(&format!("{datagroup_str}Desc"), datagroup.span())
}

/// Register the way a datagroup struct initializes
pub fn register_datagroup_init(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let info: DataGroupInitParseDesc = parse_macro_input!(args as DataGroupInitParseDesc);
    let datagroup = &info.datagroup_name;
    let datagroup_desc_trait = get_datagroup_desc_trait(&datagroup);

    let init_fn_trait = match &info.init_type {
        DataGroupInit::NoInit => quote! {},
        DataGroupInit::NoArg => quote! {fn init(&mut self);},
        DataGroupInit::Arg(arg) => {
            quote!(fn init(&mut self, init_data : Box<#arg>);)
        }
        DataGroupInit::OptionalArg(arg) => {
            quote!(fn init(&mut self, init_data : std::option::Option<Box<#arg>>);)
        }
    };

    let init_fn_arg_trait_check = match &info.init_type {
        DataGroupInit::NoInit => quote! {},
        DataGroupInit::NoArg => quote! {},
        DataGroupInit::Arg(arg) => {
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
        DataGroupInit::OptionalArg(arg) => {
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

    let init_fn_internal = match &info.init_type {
        DataGroupInit::NoInit => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::data_group::GenericDataGroupInitArg>)
            {
                panic!("Datagroup with no init!");
            }
        },
        DataGroupInit::NoArg => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::data_group::GenericDataGroupInitArg>)
            {
                assert!(_init_data.is_none(), "Unexpected init data!");
                self.init();
            }
        },
        DataGroupInit::Arg(_) => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::data_group::GenericDataGroupInitArg>)
            {
                let _init_data = _init_data.expect("Missing init data!");
                let _init_data = proto_ecs::core::casting::into_any(_init_data);
                self.init(_init_data);
            }
        },
        DataGroupInit::OptionalArg(_) => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<proto_ecs::data_group::GenericDataGroupInitArg>)
            {
                let _init_data = _init_data.and_then(|v| Some(proto_ecs::core::casting::into_any(v)));
                self.init(_init_data);
            }
        },
    };

    let init_arg_type_desc = match &info.init_type {
        DataGroupInit::NoInit => {
            quote! {type ArgType = ();}
        }
        DataGroupInit::NoArg => {
            quote! {type ArgType = ();}
        }
        DataGroupInit::Arg(arg) => {
            quote! {type ArgType = #arg;}
        }
        DataGroupInit::OptionalArg(arg) => {
            quote! {type ArgType = #arg;}
        }
    };

    let init_const_desc = match &info.init_type {
        DataGroupInit::NoInit => {
            quote! {const INIT_DESC : proto_ecs::data_group::DataGroupInitDesc = proto_ecs::data_group::DataGroupInitDesc::NoInit;}
        }
        DataGroupInit::NoArg => {
            quote! {const INIT_DESC : proto_ecs::data_group::DataGroupInitDesc = proto_ecs::data_group::DataGroupInitDesc::NoArg;}
        }
        DataGroupInit::Arg(_) => {
            quote! {const INIT_DESC : proto_ecs::data_group::DataGroupInitDesc = proto_ecs::data_group::DataGroupInitDesc::Arg;}
        }
        DataGroupInit::OptionalArg(_) => {
            quote! {const INIT_DESC : proto_ecs::data_group::DataGroupInitDesc = proto_ecs::data_group::DataGroupInitDesc::OptionalArg;}
        }
    };

    let prepare_fn = match &info.init_type {
        DataGroupInit::NoInit => {
            let msg = format!(
                "Add data group {} to an entity being prepared to spawn",
                datagroup
            );

            quote!(
                #[doc = #msg]
                pub fn prepare_spawn(spawn_desc : &mut proto_ecs::entity_spawn_desc::EntitySpawnDescription) -> std::option::Option<proto_ecs::data_group::DataGroupInitType> {
                    spawn_desc.add_datagroup::<#datagroup>(proto_ecs::data_group::DataGroupInitType::NoInit)
                }
            )
        }
        DataGroupInit::NoArg => {
            let msg = format!(
                "Add data group {} to an entity being prepared to spawn. It will init",
                datagroup
            );

            quote!(
                #[doc = #msg]
                pub fn prepare_spawn(spawn_desc : &mut proto_ecs::entity_spawn_desc::EntitySpawnDescription) -> std::option::Option<proto_ecs::data_group::DataGroupInitType> {
                    spawn_desc.add_datagroup::<#datagroup>(proto_ecs::data_group::DataGroupInitType::NoArg)
                }
            )
        }
        DataGroupInit::Arg(arg) => {
            let msg = format!(
                "Add data group {} to an entity being prepared to spawn. It will init with arg {}",
                datagroup, arg
            );

            quote!(
                #[doc = #msg]
                pub fn prepare_spawn(spawn_desc : &mut proto_ecs::entity_spawn_desc::EntitySpawnDescription, arg : Box<#arg>) -> std::option::Option<proto_ecs::data_group::DataGroupInitType> {
                    spawn_desc.add_datagroup::<#datagroup>(proto_ecs::data_group::DataGroupInitType::Arg(arg))
                }
            )
        }
        DataGroupInit::OptionalArg(arg) => {
            let msg = format!("Add data group {} to an entity being prepared to spawn. It will init with optional arg {}", datagroup, arg);

            quote!(
                #[doc = #msg]
                pub fn prepare_spawn(spawn_desc : &mut proto_ecs::entity_spawn_desc::EntitySpawnDescription, arg : std::option::Option<Box<#arg>>) -> std::option::Option<proto_ecs::data_group::DataGroupInitType> {
                    spawn_desc.add_datagroup::<#datagroup>(proto_ecs::data_group::DataGroupInitType::OptionalArg(arg))
                }
            )
        }
    };

    let result = quote! {
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
    };

    return result.into();
}

struct DatagroupInput {
    datagroup: syn::Ident,
    factory: syn::Ident,
}

impl Parse for DatagroupInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let result =
            syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated(input)?;

        if result.len() < 2 {
            return Err(syn::Error::new(
                input.span(),
                "Expected at the least two identifiers: DataGroup struct and factory function",
            ));
        }

        let datagroup = result[0].clone();
        let factory = result[1].clone();

        return Ok(DatagroupInput { datagroup, factory });
    }
}

/// Register a datagroup struct as a new datagroup class in the global registry
pub fn register_datagroup(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DatagroupInput { datagroup, factory } = parse_macro_input!(args as DatagroupInput);
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
                    Box::new(
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

    return result.into();
}
