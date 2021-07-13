use proc_macro::TokenStream;

mod imp;

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    imp::derive(input.into()).into()
}
