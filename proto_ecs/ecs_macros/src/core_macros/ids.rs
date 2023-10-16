use quote::quote;
use syn;

/// Extend a TokenStream to add implementations for ID related
/// traits for the specified struct.
/// The implementation will add a static variable of type
/// `proto_ecs::core::ids::OnceCell<proto_ecs::core::ids::ID>` where the ids will be stored.
/// The caller should provide the required code to set the right value properly.
pub fn implement_id_traits(
    struct_ident: &syn::Ident,
    program: &mut proc_macro2::TokenStream,
) -> syn::Ident {
    let struct_id_magic_ident = {
        let name_up = struct_ident.to_string().to_uppercase();
        syn::Ident::new(&format!("{name_up}_STATIC_ID"), struct_ident.span())
    };

    program.extend::<proc_macro2::TokenStream>(quote!{
        // Static value to hold id
        static #struct_id_magic_ident : proto_ecs::core::ids::OnceCell<proto_ecs::core::ids::ID> = proto_ecs::core::ids::OnceCell::new();

        // Implement locator trait for registry,
        // it helps you to find the id for a struct using static function calls
        impl proto_ecs::core::ids::IDLocator for #struct_ident
        {
            #[inline(always)]
            fn get_id() -> proto_ecs::core::ids::ID
            {
                #struct_id_magic_ident.get().expect("Missing ID").clone()
            }
        }

        // Implement metadata trait for this struct. It helps you to
        // get the id of a struct instance, so that you can find its
        // static data with the global registry
        impl proto_ecs::core::ids::HasID for #struct_ident
        {
            #[inline(always)]
            fn get_id(&self) -> proto_ecs::core::ids::ID
            {
                #struct_id_magic_ident.get().expect("Missing ID").clone()
            }
        }
    }.into());

    return struct_id_magic_ident;
}
