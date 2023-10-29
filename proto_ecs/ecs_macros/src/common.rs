use proc_macro;
use quote::quote;
use syn;

/// The style of argument for an init function.
/// If it has one, it can specify if it doesn't take an argument,
/// if the argument is required, or if the argument is optional
pub enum InitArgStyle
{
    /// No Init
    NoInit,
    /// Init with no Args
    NoArg,
    /// Init with Args
    Arg(syn::Ident),
    /// Init with optional Args
    OptionalArg(syn::Ident),
}

impl syn::parse::Parse for InitArgStyle
{
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let init_type: syn::Ident = input.parse()?;

        match init_type.to_string().as_str() {
            "NoInit" => Ok(InitArgStyle::NoInit),
            "NoArg" => Ok(InitArgStyle::NoArg),
            "Arg" => {
                let content;
                let _ : syn::token::Paren = syn::parenthesized!(content in input); // parenthesis
                let arg_type : syn::Ident = content.parse()?;
                Ok(InitArgStyle::Arg(arg_type))
            },
            "OptionalArg" => {
                let content;
                let _ : syn::token::Paren = syn::parenthesized!(content in input); // parenthesis
                let arg_type : syn::Ident = content.parse()?;
                Ok(InitArgStyle::OptionalArg(arg_type))
            },
            unknown => Err(
                syn::Error::new(init_type.span(), format!("Unexpected Init type \'{unknown}\'. The only valids are: NoInit, NoArg, Arg(ArgType), OptionalArg(ArgType)"))
            )
        }
    }
}

impl InitArgStyle
{
    pub fn to_signature(&self) -> proc_macro2::TokenStream
    {
        match self {
            InitArgStyle::NoInit => quote! {},
            InitArgStyle::NoArg => quote! {fn init(&mut self);},
            InitArgStyle::Arg(arg) => {
                quote!(fn init(&mut self, init_data : std::boxed::Box<#arg>);)
            }
            InitArgStyle::OptionalArg(arg) => {
                quote!(fn init(&mut self, init_data : std::option::Option<std::boxed::Box<#arg>>);)
            }
        }
    }
}

pub fn _ensure_struct_implements_trait(struct_id : syn::Ident, trait_id : syn::Ident, result : &mut proc_macro::TokenStream)
{
    result.extend::<proc_macro::TokenStream>(quote!{
        const _ : fn() = || {
            fn check_item_implements_trait<T: ?Sized + #trait_id>(){};
            check_item_implements_trait::<#struct_id>();
        };
    }.into());
}
