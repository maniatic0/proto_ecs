use proc_macro;
use quote::quote;
use std::sync::atomic::{AtomicU32, Ordering};
use syn::{DeriveInput, parse_macro_input, self, parse::Parse};
use crc32fast;

// -- < Datagroups > -----------------------------------
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
                proto_ecs::data_group::DataGroupRegistry::get_global_registry()
                    .lock()
                    .as_mut()
                    .and_then(
                        |registry|
                        {
                            registry.register(proto_ecs::data_group::DataGroupRegistryEntry{
                                name: #datagroup_str,
                                name_crc: #name_crc,
                                factory_func: #factory,
                                id: #id
                            });

                            Ok(())
                        }
                    ).expect("Can't access registry due to poisoning");
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