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
/// Register a global system in the system registry. 
/// The first argument should be the name of the struct to register as 
/// global system. Then follows a list keyword arguments:
/// 
/// * `dependencies`: (optional) List of datagroups that should be present in all entities subscribed to this global system
/// * `stages`: (optional) List of stages that this datagroup should run on. If no stage is specified, it won't ever run
/// * `before` : (optional) List of global systems that should run after this system. (Datagroup runs BEFORE ...)
/// * `after` : (optional) List of global systems that should run before this system. (Datagroup runs AFTER ...)
/// * `init_arg` : (optional) argument consumed by the initialization function to init this system. Possible options:
///     * `NoInit`: No initialization function is required.
///     * `NoArg` : Can init without arguments (default)
///     * `Arg(T)` : Init function expects a single argument of type T
///     * `OptionalArg(T)` : Init function expects an argument of type Option<T>
/// * `factory` : A function name to use as factory function. It will return an instance of `Box<dyn GlobalSystem>`
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
