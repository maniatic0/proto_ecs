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