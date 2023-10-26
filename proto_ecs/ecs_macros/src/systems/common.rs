/// Common types for both local and global systems, and some useful 
/// macro implementations.

use syn;
use quote::{quote, ToTokens};


pub enum OptionalDep {
    Dependency(syn::Ident),
    OptionalDep(syn::Ident),
}

impl OptionalDep {
    pub fn unwrap(&self) -> &syn::Ident {
        match self {
            OptionalDep::Dependency(d) => d,
            OptionalDep::OptionalDep(d) => d,
        }
    }
}

// This structs serve as "new_type", so we can avoid implementing a trait outside
// our crate for a struct outside our crate
pub struct Dependencies(pub Vec<OptionalDep>);
pub struct Stages(pub Vec<syn::LitInt>);

pub struct DependencyList(pub Vec<syn::Ident>);

impl syn::parse::Parse for OptionalDep {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let first_token = input.parse::<syn::Ident>()?;
        let first_token_str = first_token.to_string();

        match first_token_str.as_str() {
            "Optional" => {
                // parse content: Optional(SomeIdent)
                let content;
                let _ = syn::parenthesized!(content in input); // Parenthesis
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
        let _ = syn::parenthesized!(content in input); // Parenthesis

        // Parse a comma separated list of OptionalIdent
        let deps =
            syn::punctuated::Punctuated::<OptionalDep, syn::Token![,]>::parse_terminated(&content)?;

        Ok(Dependencies(deps.into_iter().collect()))
    }
}

impl syn::parse::Parse for Stages {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let _ = syn::parenthesized!(content in input);
        let stages =
            syn::punctuated::Punctuated::<syn::LitInt, syn::Token![,]>::parse_terminated(&content)?;

        Ok(Stages(stages.into_iter().collect()))
    }
}

impl syn::parse::Parse for DependencyList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let _ = syn::parenthesized!(content in input);
        let dependency_list =
            syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated(&content)?;

        Ok(DependencyList(dependency_list.into_iter().collect()))
    }
}


impl ToTokens for OptionalDep {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            OptionalDep::Dependency(id) => {
                tokens.extend(quote! {
                    proto_ecs::systems::common::Dependency::DataGroup(
                        <#id as proto_ecs::core::ids::IDLocator>::get_id()
                    )
                });
            }
            OptionalDep::OptionalDep(id) => {
                tokens.extend(quote! {
                    proto_ecs::systems::common::Dependency::OptionalDG(
                        <#id as proto_ecs::core::ids::IDLocator>::get_id()
                    )
                });
            }
        };
    }
}
