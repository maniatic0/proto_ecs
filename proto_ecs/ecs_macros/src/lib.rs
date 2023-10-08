use proc_macro;
use proc_macro2::{TokenStream, token_stream};
use quote::{quote, ToTokens};
use std::sync::atomic::{AtomicU32, Ordering};
use syn::{DeriveInput, parse_macro_input, self, parse::Parse, token, parenthesized, spanned::Spanned};
use crc32fast;

// -- < Datagroups > -----------------------------------

/// Whether a DataGroup has an init function
/// If it has one, it can specify if it doesn't take an argument, 
/// if the argument is required, or if the argument is optional
enum DataGroupInit {
    None,
    NoArg,
    Arg(syn::Ident),
    OptionalArg(syn::Ident)
}

struct DataGroupInitParseDesc {
    pub datagroup_name : syn::Ident,
    pub init_type : DataGroupInit
}

impl Parse for DataGroupInitParseDesc
{
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
       
       let datagroup_name : syn::Ident = input.parse()?;
       let _ : token::Comma = input.parse()?; // comma token
       let init_type : syn::Ident = input.parse()?;

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
fn get_datagroup_desc_trait(datagroup : &syn::Ident) -> syn::Ident
{
    let datagroup_str = datagroup.to_string();
    syn::Ident::new(&format!("{datagroup_str}Desc"), datagroup.span())
}

/// Register the way a datagroup struct initializes
#[proc_macro]
pub fn register_datagroup_init(args : proc_macro::TokenStream) -> proc_macro::TokenStream
{
    let info : DataGroupInitParseDesc = parse_macro_input!(args as DataGroupInitParseDesc);
    let datagroup_desc_trait = get_datagroup_desc_trait(&info.datagroup_name);

    let init_fn_trait = match &info.init_type {
        DataGroupInit::None => quote!{},
        DataGroupInit::NoArg => quote!{fn init(&mut self);},
        DataGroupInit::Arg(arg) => {
            let arg_clone = arg.clone();
            quote!(fn init(&mut self, init_data : Box<#arg_clone>);)
        },
        DataGroupInit::OptionalArg(arg) => {
            let arg_clone = arg.clone();
            quote!(fn init(&mut self, init_data : std::option::Option<Box<#arg_clone>>);)
        },
    };

    let init_fn_arg_trait_check = match &info.init_type {
        DataGroupInit::None => quote!{},
        DataGroupInit::NoArg => quote!{},
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
        },
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
        },
    };

    let init_fn_internal = match info.init_type {
        DataGroupInit::None => quote!{
            fn __init__(&mut self, _init_data: std::option::Option<Box<dyn CanCast>>)
            {
                panic!("Datagroup with no init!");
            }
        },
        DataGroupInit::NoArg => quote!{
            fn __init__(&mut self, _init_data: std::option::Option<Box<dyn CanCast>>)
            {
                assert!(_init_data.is_none(), "Unexpected init data!");
                self.init();
            }
        },
        DataGroupInit::Arg(arg) => quote!{
            fn __init__(&mut self, _init_data: std::option::Option<Box<dyn CanCast>>)
            {
                let _init_data = _init_data.expect("Missing init data!");
                let _init_data = proto_ecs::core::casting::into_any::<#arg>(_init_data);
                self.init(_init_data);
            }
        },
        DataGroupInit::OptionalArg(arg) => quote!{
            fn __init__(&mut self, _init_data: std::option::Option<Box<dyn CanCast>>)
            {
                let _init_data = _init_data.and_then(|v| Some(proto_ecs::core::casting::into_any::<#arg>(v)));
                self.init(_init_data);
            }
        },
    };

    let datagroup = &info.datagroup_name;

    let result = quote!{
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

struct DatagroupInput
{
    datagroup : syn::Ident,
    factory : syn::Ident
}

impl Parse for DatagroupInput
{
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let result = syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated(input)?;
        
        if result.len() < 2 
        {
            return Err(syn::Error::new(input.span(), "Expected at the least two identifiers: DataGroup struct and factory function"));
        }

        let datagroup = result[0].clone();
        let factory = result[1].clone();
        
        return Ok(DatagroupInput{datagroup, factory});
    }
}

static DATAGROUP_COUNT : AtomicU32 = AtomicU32::new(0);

/// Register a datagroup struct as a new datagroup class in the global registry
#[proc_macro]
pub fn register_datagroup(args : proc_macro::TokenStream) -> proc_macro::TokenStream
{
    let DatagroupInput { datagroup, factory } = parse_macro_input!(args as DatagroupInput);
    let datagroup_str = datagroup.to_string();
    let name_crc = crc32fast::hash(datagroup_str.as_bytes());
    let id = DATAGROUP_COUNT.fetch_add(1, Ordering::Relaxed);
    let datagroup_desc_trait = get_datagroup_desc_trait(&datagroup);

    let result = quote!{

        const _: fn() = || {
            /// Only callable when Datagroup implements trait DatagroupDesc.
            fn check_desc_trait_implemented<T: ?Sized + #datagroup_desc_trait>() {}
            check_desc_trait_implemented::<#datagroup>();
            // Based on https://docs.rs/static_assertions/latest/static_assertions/macro.assert_impl_all.html
        };

        // Registration in the global datagroup registry
        const _ : () = {
            #[ctor::ctor]
            fn __register_datagroup__()
            {
                let mut global_registry = proto_ecs::data_group::DataGroupRegistry::get_global_registry().write();
                global_registry.register(proto_ecs::data_group::DataGroupRegistryEntry{
                    name: #datagroup_str,
                    name_crc: #name_crc,
                    factory_func: #factory,
                    id: #id
                });
            }
        };

        // Implement locator trait for registry, 
        // it helps you to find the id for a datagroup using static function calls
        impl proto_ecs::data_group::DataGroupMetadataLocator for #datagroup
        {
            fn get_id() -> proto_ecs::data_group::DataGroupID
            {
                #id
            }
        }

        // Implement metadata trait for this datagroup. It helps you to 
        // get the id of a datagroup instance, so that you can find its 
        // static data with the global registry
        impl proto_ecs::data_group::DataGroupMeta for #datagroup
        {
            fn get_id(&self) -> proto_ecs::data_group::DataGroupID
            {
                #id
            }
        }
    };


    return result.into();
}

// -- < Entities > ---------------------------------------------------

static ENTITY_CLASS_COUNT : AtomicU32 = AtomicU32::new(0);

#[proc_macro_attribute]
pub fn entity(attr : proc_macro::TokenStream, item : proc_macro::TokenStream) -> proc_macro::TokenStream
{
    assert!(attr.is_empty());
    let mut item_copy = item.clone();
    let DeriveInput {ident, ..} = parse_macro_input!(item);
    let next_id = ENTITY_CLASS_COUNT.fetch_add(1, Ordering::Relaxed);
    let trait_impls = quote!{
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
    return  item_copy;
}

// -- < Local systems > --------------------------------------
static LOCAL_SYSTEM_COUNT : AtomicU32 = AtomicU32::new(1);

#[proc_macro_attribute]
pub fn local_system(_args : proc_macro::TokenStream, item : proc_macro::TokenStream) -> proc_macro::TokenStream
{
    let mut item_copy = item.clone();
    let syn::ItemFn{sig, ..} = parse_macro_input!(item);

    let mut arg_types = vec![];
    for arg in sig.inputs.iter()
    {
        // Args should be a mutable reference to a datagroup
        match arg {
            syn::FnArg::Typed(pt) => { 
                match (*pt.ty).clone() {
                    syn::Type::Reference(r) => {
                        match (*r.elem).clone() {
                            syn::Type::Path(p) => {arg_types.push(p)}
                            _ => { return quote!{compile_error!("Systems should only expect DataGroups as input")}.into(); }
                        }
                    },
                    _ => { return quote!{compile_error!("Systems should only expect DataGroups as input")}.into(); }
                }
            },
            _ => { return quote!(compile_error!("Systems should only expect DataGroups as input")).into();}
        }
    }

    // Create header of glue function. This function will be used to run
    // the user specified function
    let func_name = format!("__{}__", sig.ident.to_string());
    let mut  new_function = format!(
            " fn {}(indices : &[usize], entity_datagroups : &mut Vec<std::boxed::Box<dyn proto_ecs::data_group::DataGroup>>)
            {{ 
                debug_assert!({{
                    let mut unique_set = std::collections::HashSet::new();
                    indices.iter().all(|&i| {{unique_set.insert(i) && i < entity_datagroups.len()}})
                }}, \"Overlapping indices or index out of range\");
                unsafe
                {{
                let entity_datagroups_ptr = entity_datagroups.as_mut_ptr();
            ", func_name);

    // Add code for auto casting
    for i in 0..arg_types.len()
    {
        new_function += format!(
            "let arg{} = (&mut *entity_datagroups_ptr.add(indices[{}]))
                        .as_any_mut()
                        .downcast_mut::<{}>()
                        .expect(\"Cast is not possible\");\n", 
            i, i, 
            arg_types[i]
                .path
                .clone()
                .into_token_stream()
                .to_string()
        ).as_str(); // I hope there's a simpler way to do this, too many string and clones
    }
    
    // Write the final call to the user function
    new_function += format!("{}(", sig.ident.to_string()).as_str();

    for i in 0..arg_types.len()
    {
        new_function += format!("arg{i}").as_str();
        if i != arg_types.len() - 1
        {
            new_function += ",";
        }
    }
    new_function += ");}}\n";

    // Append this function to the end of the original function
    item_copy.extend::<proc_macro::TokenStream>(new_function.as_str().parse().unwrap());

    // Add this new system to the global system register
    let id = LOCAL_SYSTEM_COUNT.fetch_add(1, Ordering::Relaxed);
    let name_crc = crc32fast::hash(sig.ident.to_string().as_bytes());
    let deps = arg_types.iter().map(|ty| {&ty.path});
    let func_ident = syn::Ident::new(func_name.as_str(), sig.span());
    item_copy.extend::<proc_macro::TokenStream>(quote!(
        const _ : () = {
            #[ctor::ctor]
            fn __register_local_system__()
            {
                let mut registry = proto_ecs::local_systems::LocalSystemRegistry::get_global_registry().write();
                let mut dependencies = Vec::new();
                #( dependencies.push(<#deps as proto_ecs::data_group::DataGroupMetadataLocator>::get_id());)*
                registry.register(
                    proto_ecs::local_systems::LocalSystemRegistryEntry{
                        id : #id,
                        name_crc : #name_crc,
                        dependencies : dependencies,
                        func : #func_ident
                    }
                );
            }
        };
    ).into());

    return item_copy;
}

// -- < Misc macros > ----------------------------------------

#[proc_macro_derive(CanCast)]
pub fn derive_can_cast(item : proc_macro::TokenStream) -> proc_macro::TokenStream
{
    let DeriveInput { ident, .. } = parse_macro_input!(item);
    
    
    return quote!{
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
    }.into();
}