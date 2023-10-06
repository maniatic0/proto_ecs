use proc_macro;
use quote::quote;
use std::sync::atomic::{AtomicU32, Ordering};
use syn::{DeriveInput, parse_macro_input, self, parse::Parse, token, parenthesized};
use crc32fast;

// -- < Datagroups > -----------------------------------

/// Whether a DataGroup has an init function
/// If it has one, it can specify if it doesn't take an argument, if the argument is required, or if the argument is optional
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

/// Register the way a datagroup struct initializes
#[proc_macro]
pub fn register_datagroup_init(args : proc_macro::TokenStream) -> proc_macro::TokenStream
{
    let info : DataGroupInitParseDesc = parse_macro_input!(args as DataGroupInitParseDesc);
    let datagroup_str = info.datagroup_name.to_string();
    let name_crc = crc32fast::hash(datagroup_str.as_bytes());
    let datagroup_trait = syn::Ident::new(&format!("{datagroup_str}Desc"), info.datagroup_name.span());

    let init_fn = match info.init_type {
        DataGroupInit::None => quote!{},
        DataGroupInit::NoArg => quote!{fn init(&mut self);},
        DataGroupInit::Arg(arg) => {
            quote!(fn init(&mut self, arg : #arg);)
        },
        DataGroupInit::OptionalArg(arg) => {
            quote!(fn init(&mut self, arg : std::option::Option<#arg>);)
        },
    };

    let result = quote!{
        trait #datagroup_trait {
            #init_fn
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

    let result = quote!{

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

// -- < Misc macros > ----------------------------------------

#[proc_macro_derive(CanCast)]
pub fn derive_can_cast(item : proc_macro::TokenStream) -> proc_macro::TokenStream
{
    let DeriveInput { ident, .. } = parse_macro_input!(item);
    
    
    return quote!{
        impl proto_ecs::core::casting::CanCast for #ident
        {
            
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