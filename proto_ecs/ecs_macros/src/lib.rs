use proc_macro;
use quote::quote;
use std::sync::atomic::{AtomicU32, Ordering};
use syn::{
    self, DeriveInput, parse_macro_input,
};

mod tests;
mod utils;


// -- < Datagroups > -----------------------------------
mod datagroup_macros;

/// Register the way a datagroup struct initializes
#[proc_macro]
pub fn register_datagroup_init(args: proc_macro::TokenStream) -> proc_macro::TokenStream
{
    datagroup_macros::register_datagroup_init(args)
}

/// Register a datagroup struct as a new datagroup class in the global registry
#[proc_macro]
pub fn register_datagroup(args: proc_macro::TokenStream) -> proc_macro::TokenStream 
{
    datagroup_macros::register_datagroup(args)
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

mod local_systems_macros;

/// Register a struct as a local system. 
/// 
/// Example usage:
/// ```ignore
/// struct Example;
/// 
/// register_local_system!{
///     Example,
///     dependencies = (DataGroup1, Optional(DataGroup2)),
///     stages = (0,1)
/// }
/// 
/// impl ExampleLocalSystem for Example
/// {
///     fn stage_0(dg1 : &mut DataGroup1, dg2 : Option<&mut DataGroup2>)
///     { todo!()}
/// 
///     fn stage_1(dg1 : &mut DataGroup1, dg2 : Option<&mut DataGroup2>)
///     { todo!()}
/// }
/// ```
#[proc_macro]
pub fn register_local_system(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    local_systems_macros::register_local_system(input)
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
