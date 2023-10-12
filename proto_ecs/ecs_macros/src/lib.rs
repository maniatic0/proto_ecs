use crc32fast;
use proc_macro;
use quote::{quote, ToTokens};
use std::sync::atomic::{AtomicU32, Ordering};
use syn::{
    self, parenthesized, parse::Parse, parse_macro_input, spanned::Spanned, token, DeriveInput,
};

mod tests;
mod utils;

// -- < Datagroups > -----------------------------------

/// Whether a DataGroup has an init function
/// If it has one, it can specify if it doesn't take an argument,
/// if the argument is required, or if the argument is optional
enum DataGroupInit {
    None,
    NoArg,
    Arg(syn::Ident),
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
        "None" => Ok(DataGroupInitParseDesc{ datagroup_name : datagroup_name.clone(), init_type : DataGroupInit::None}),
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
        _ => Err(
            syn::Error::new(init_type.span(), "Unexpected Init type. The only valids are: None, NoArg, Arg(ArgType), OptionalArg(ArgType)")
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
#[proc_macro]
pub fn register_datagroup_init(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let info: DataGroupInitParseDesc = parse_macro_input!(args as DataGroupInitParseDesc);
    let datagroup_desc_trait = get_datagroup_desc_trait(&info.datagroup_name);

    let init_fn_trait = match &info.init_type {
        DataGroupInit::None => quote! {},
        DataGroupInit::NoArg => quote! {fn init(&mut self);},
        DataGroupInit::Arg(arg) => {
            let arg_clone = arg.clone();
            quote!(fn init(&mut self, init_data : Box<#arg_clone>);)
        }
        DataGroupInit::OptionalArg(arg) => {
            let arg_clone = arg.clone();
            quote!(fn init(&mut self, init_data : std::option::Option<Box<#arg_clone>>);)
        }
    };

    let init_fn_arg_trait_check = match &info.init_type {
        DataGroupInit::None => quote! {},
        DataGroupInit::NoArg => quote! {},
        DataGroupInit::Arg(arg) => {
            let arg_clone = arg.clone();
            quote!(
                const _: fn() = || {
                    /// Only callable when Arg implements trait CanCast.
                    fn check_cast_trait_implemented<T: ?Sized + proto_ecs::core::casting::CanCast>() {}
                    check_cast_trait_implemented::<#arg_clone>();
                    // Based on https://docs.rs/static_assertions/latest/static_assertions/macro.assert_impl_all.html
                };
            )
        }
        DataGroupInit::OptionalArg(arg) => {
            let arg_clone = arg.clone();
            quote!(
                const _: fn() = || {
                    /// Only callable when Arg implements trait CanCast.
                    fn check_cast_trait_implemented<T: ?Sized + proto_ecs::core::casting::CanCast>() {}
                    check_cast_trait_implemented::<#arg_clone>();
                    // Based on https://docs.rs/static_assertions/latest/static_assertions/macro.assert_impl_all.html
                };
            )
        }
    };

    let init_fn_internal = match info.init_type {
        DataGroupInit::None => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<Box<dyn CanCast>>)
            {
                panic!("Datagroup with no init!");
            }
        },
        DataGroupInit::NoArg => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<Box<dyn CanCast>>)
            {
                assert!(_init_data.is_none(), "Unexpected init data!");
                self.init();
            }
        },
        DataGroupInit::Arg(_) => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<Box<dyn CanCast>>)
            {
                let _init_data = _init_data.expect("Missing init data!");
                let _init_data = proto_ecs::core::casting::into_any(_init_data);
                self.init(_init_data);
            }
        },
        DataGroupInit::OptionalArg(_) => quote! {
            fn __init__(&mut self, _init_data: std::option::Option<Box<dyn CanCast>>)
            {
                let _init_data = _init_data.and_then(|v| Some(proto_ecs::core::casting::into_any(v)));
                self.init(_init_data);
            }
        },
    };

    let datagroup = &info.datagroup_name;

    let result = quote! {
        #init_fn_arg_trait_check
        trait #datagroup_desc_trait {
            #init_fn_trait
        }

        impl proto_ecs::data_group::DataGroup for #datagroup
        {
            #init_fn_internal
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
#[proc_macro]
pub fn register_datagroup(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DatagroupInput { datagroup, factory } = parse_macro_input!(args as DatagroupInput);
    let datagroup_str = datagroup.to_string();
    let name_crc = crc32fast::hash(datagroup_str.as_bytes());
    let datagroup_desc_trait = get_datagroup_desc_trait(&datagroup);
    let datagroup_id_magic_ident = {
        let name_up = datagroup_str.to_uppercase();
        syn::Ident::new(&format!("{name_up}_DATA_GROUP_ID"), datagroup.span())
    };

    let result = quote! {

        const _: fn() = || {
            /// Only callable when Datagroup implements trait DatagroupDesc.
            fn check_desc_trait_implemented<T: ?Sized + #datagroup_desc_trait>() {}
            check_desc_trait_implemented::<#datagroup>();
            // Based on https://docs.rs/static_assertions/latest/static_assertions/macro.assert_impl_all.html
        };

        // Static value to hold id
        static #datagroup_id_magic_ident : proto_ecs::data_group::OnceCell<proto_ecs::data_group::DataGroupID> = proto_ecs::data_group::OnceCell::new();

        // Registration in the global datagroup registry
        const _ : () = {
            #[ctor::ctor]
            fn __register_datagroup__()
            {
                proto_ecs::data_group::DataGroupRegistry::register_lambda(
                    Box::new(
                        |registry| {
                            let new_id = registry.register(proto_ecs::data_group::DataGroupRegistryEntry{
                                name: #datagroup_str,
                                name_crc: #name_crc,
                                factory_func: #factory,
                                id: proto_ecs::data_group::DataGroupID::MAX
                            });
                            #datagroup_id_magic_ident.set(new_id).expect("Failed to register DataGroup ID");
                        }
                    )
                );
            }
        };

        // Implement locator trait for registry,
        // it helps you to find the id for a datagroup using static function calls
        impl proto_ecs::data_group::DataGroupMetadataLocator for #datagroup
        {
            fn get_id() -> proto_ecs::data_group::DataGroupID
            {
                #datagroup_id_magic_ident.get().expect("Missing DataGroup ID").clone()
            }
        }

        // Implement metadata trait for this datagroup. It helps you to
        // get the id of a datagroup instance, so that you can find its
        // static data with the global registry
        impl proto_ecs::data_group::DataGroupMeta for #datagroup
        {
            fn get_id(&self) -> proto_ecs::data_group::DataGroupID
            {
                #datagroup_id_magic_ident.get().expect("Missing DataGroup ID").clone()
            }
        }
    };

    return result.into();
}

// -- < Entities > ---------------------------------------------------

static ENTITY_CLASS_COUNT: AtomicU32 = AtomicU32::new(0);

#[proc_macro_attribute]
pub fn entity(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    assert!(attr.is_empty());
    let mut item_copy = item.clone();
    let DeriveInput { ident, .. } = parse_macro_input!(item);
    let next_id = ENTITY_CLASS_COUNT.fetch_add(1, Ordering::Relaxed);
    let trait_impls = quote! {
        impl EntityDesc for #ident
        {
            fn get_class_id() -> EntityClassID
            {
                #next_id
            }

            fn get_class_name() -> &'static str
            {
                std::any::type_name::<#ident>()
            }
        }

    };

    item_copy.extend::<proc_macro::TokenStream>(trait_impls.into());
    return item_copy;
}

// -- < Local systems > --------------------------------------
static LOCAL_SYSTEM_COUNT: AtomicU32 = AtomicU32::new(1);

// This structs serve as "new_type", so we can avoid implementing a trait outside
// our crate for a struct outside our crate
enum OptionalDep {
    Dependency(syn::Ident),
    OptionalDep(syn::Ident),
}

impl OptionalDep {
    fn unwrap(&self) -> &syn::Ident {
        match self {
            OptionalDep::Dependency(d) => d,
            OptionalDep::OptionalDep(d) => d,
        }
    }
}

struct Dependencies(Vec<OptionalDep>);
struct Stages(Vec<syn::LitInt>);

struct LocalSystemArgs {
    struct_id: syn::Ident,
    dependencies: Dependencies,
    stages: Stages,
}

impl syn::parse::Parse for OptionalDep {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let first_token = input.parse::<syn::Ident>()?;
        let first_token_str = first_token.to_string();

        match first_token_str.as_str() {
            "Optional" => {
                // parse content: Optional(SomeIdent)
                let content;
                let _ = parenthesized!(content in input); // Parenthesis
                let inner_ident = content.parse::<syn::Ident>()?;
                return Ok(OptionalDep::OptionalDep(inner_ident));
            }

            _ => {
                // A bare id: SomeIdent
                return Ok(OptionalDep::Dependency(first_token));
            }
        }
    }
}

impl syn::parse::Parse for Dependencies {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let _ = parenthesized!(content in input); // Parenthesis

        // Parse a comma separated list of OptionalIdent
        let deps =
            syn::punctuated::Punctuated::<OptionalDep, syn::Token![,]>::parse_terminated(&content)?;

        Ok(Dependencies(deps.into_iter().collect()))
    }
}

impl syn::parse::Parse for Stages {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let _ = parenthesized!(content in input);
        let stages =
            syn::punctuated::Punctuated::<syn::LitInt, syn::Token![,]>::parse_terminated(&content)?;

        Ok(Stages(stages.into_iter().collect()))
    }
}

impl ToTokens for OptionalDep {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            OptionalDep::Dependency(id) => {
                tokens.extend(quote! {
                    proto_ecs::local_systems::Dependency::DataGroup(
                        <#id as proto_ecs::data_group::DataGroupMetadataLocator>::get_id()
                    )
                });
            }
            OptionalDep::OptionalDep(id) => {
                tokens.extend(quote! {
                    proto_ecs::local_systems::Dependency::OptionalDG(
                        <#id as proto_ecs::data_group::DataGroupMetadataLocator>::get_id()
                    )
                });
            }
        };
    }
}

impl syn::parse::Parse for LocalSystemArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let struct_id = input.parse::<syn::Ident>()?;
        let _ = input.parse::<token::Comma>()?;
        let mut dependencies: Option<Dependencies> = None;
        let mut stages: Option<Stages> = None;

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
        format!("__{}__", function_id.to_string()).as_str(),
        function_id.span(),
    );

    let arg_ids =
        (0..args.len()).map(|i| syn::Ident::new(format!("arg{i}").as_str(), function_id.span()));

    // required to prevent use-after-move error later on this function
    let arg_ids_copy = arg_ids.clone();

    let arg_values = args.iter().enumerate().map(|(i, arg)| {
        let index = syn::Index::from(i);
        let type_id = arg.unwrap();
        let arg_value = quote! {
            (&mut *entity_datagroups_ptr.add(indices[#index]))
            .as_any_mut()
            .downcast_mut::<#type_id>()
            .expect("Couldn't perform cast")
        };

        match arg {
            OptionalDep::OptionalDep(_) => {
                quote! {
                    if indices[#index] == usize::MAX
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
        fn #new_function_id(indices : &[usize], entity_datagroups : &mut Vec<std::boxed::Box<dyn proto_ecs::data_group::DataGroup>>)
        {
            debug_assert!({
                let mut unique_set = std::collections::HashSet::new();
                indices.iter().all(|&i| {{unique_set.insert(i) && i < entity_datagroups.len()}})
            }, "Overlapping indices or index out of range");

            unsafe {
                let entity_datagroups_ptr = entity_datagroups.as_mut_ptr();
                #(let #arg_ids = #arg_values;)*
                #struct_id :: #function_id (#( #arg_ids_copy, )*);
            }
        }
    };

    return (new_function_id, new_function);
}

#[proc_macro]
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

    // Generate function arguments for trait functions
    let function_args = deps
        .iter()
        .map(|dep| {
            let to_arg_name = |d: &syn::Ident| {
                let ident_str = utils::to_camel_case(d.to_string().as_str());
                return syn::Ident::new(ident_str.as_str(), d.span());
            };

            match dep {
                OptionalDep::Dependency(d) => {
                    let arg_name = to_arg_name(d);
                    quote! { #arg_name : &mut #d }
                }
                OptionalDep::OptionalDep(d) => {
                    let arg_name = to_arg_name(d);
                    quote! { #arg_name : &mut Option<#d> }
                }
            }
        })
        .collect::<Vec<proc_macro2::TokenStream>>();

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

    return quote!{

        // For static assertions
        const _ : fn() = || {
            fn check_implements_traits<T : #new_trait_id>(){};
            check_implements_traits::<#struct_id>();
        };

        // Generate the trait to be implemented by the user 
        pub trait #new_trait_id 
        {
           #(#function_signatures;)*
        }

        #(#glue_function_bodies)*

        // Register this new local system to be loaded later
        const _ : () =
        {
            #[ctor::ctor]
            fn __register_local_system__()
            {
                proto_ecs::local_systems::LocalSystemRegistry::register_lambda(
                    Box::new(
                        |registry| {
                            let mut dependencies = Vec::new();
                            let mut func_map : [Option<proto_ecs::local_systems::SystemFn>; 255] = [None; 255];
                            #( dependencies.push(#deps);)*
                            #( func_map[#stage_indices] = Some(#glue_function_ids);)*
                            registry.register_internal(
                                proto_ecs::local_systems::LocalSystemRegistryEntry{
                                    id : u32::MAX,
                                    name_crc : #name_crc,
                                    dependencies : dependencies,
                                    functions : func_map
                                }
                            );
                        }
                    )
                );
            }
        };
    }.into();
}

// -- < Misc macros > ----------------------------------------

#[proc_macro_derive(CanCast)]
pub fn derive_can_cast(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(item);

    return quote! {
        impl proto_ecs::core::casting::CanCast for #ident
        {
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>
            {
                self
            }

            fn as_any(&self) -> &dyn std::any::Any
            {
                self as &dyn std::any::Any
            }
            fn as_any_mut(&mut self) ->&mut dyn std::any::Any
            {
                self as &mut dyn std::any::Any
            }

        }
    }
    .into();
}
