
#[derive(Debug, PartialEq)]
/// Whether a something has an init function
/// If it has one, it can specify if it doesn't take an argument,
/// if the argument is required, or if the argument is optional
pub enum InitDesc {
    /// without init
    NoInit,
    /// with init but no args
    NoArg,
    /// with init and args
    Arg,
    /// with init and optional args
    OptionalArg,
}