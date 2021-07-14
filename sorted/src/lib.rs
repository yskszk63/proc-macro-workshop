use proc_macro::TokenStream;

mod imp;

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    imp::sorted(args.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn check(args: TokenStream, input: TokenStream) -> TokenStream {
    imp::check(args.into(), input.into()).into()
}
