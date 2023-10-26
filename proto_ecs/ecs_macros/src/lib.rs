use proc_macro;
use quote::quote;
use syn::{self, parse_macro_input, DeriveInput};

mod core_macros;
mod tests;
mod utils;
mod systems;
mod common;

// -- < Datagroups > -----------------------------------
mod datagroup_macros;

/// Register the way a datagroup struct initializes
#[proc_macro]
pub fn register_datagroup_init(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    datagroup_macros::register_datagroup_init(args)
}

/// Register a datagroup struct as a new datagroup class in the global registry
#[proc_macro]
pub fn register_datagroup(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    datagroup_macros::register_datagroup(args)
}

// -- < Local systems > --------------------------------------

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
    systems::local_systems_macros::register_local_system(input)
}

// -- < Global Systems Macros > ------------------------------

#[proc_macro]
pub fn register_global_system(args : proc_macro::TokenStream) -> proc_macro::TokenStream
{
    systems::global_systems_macros::register_global_system(args)
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
