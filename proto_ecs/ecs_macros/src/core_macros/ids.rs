use syn;
use quote::quote;

/// Extend a TokenStream to add implementations for ID related 
/// traits for the specified struct. 
/// The implementation will add a static variable of type 
/// `proto_ecs::core::ids::OnceCell<proto_ecs::core::ids::ID>` where the ids will be stored.
/// The caller should provide the required code to set the right value properly.
pub fn implement_id_traits(struct_ident : &syn::Ident, program : &mut proc_macro2::TokenStream) -> syn::Ident
{
    let datagroup_id_magic_ident = {
        let name_up = struct_ident.to_string().to_uppercase();
        syn::Ident::new(&format!("{name_up}_DATA_GROUP_ID"), struct_ident.span())
    };

program.extend::<proc_macro2::TokenStream>(quote!{
        // Static value to hold id
        static #datagroup_id_magic_ident : proto_ecs::core::ids::OnceCell<proto_ecs::core::ids::ID> = proto_ecs::core::ids::OnceCell::new();

        // Implement locator trait for registry,
        // it helps you to find the id for a datagroup using static function calls
        impl proto_ecs::core::ids::IDLocator for #struct_ident
        {
            fn get_id() -> proto_ecs::core::ids::ID
            {
                #datagroup_id_magic_ident.get().expect("Missing ID").clone()
            }
        }

        // Implement metadata trait for this datagroup. It helps you to
        // get the id of a datagroup instance, so that you can find its
        // static data with the global registry
        impl proto_ecs::core::ids::HasID for #struct_ident
        {
            fn get_id(&self) -> proto_ecs::core::ids::ID
            {
                #datagroup_id_magic_ident.get().expect("Missing ID").clone()
            }
        }
    }.into());

    return datagroup_id_magic_ident;
}