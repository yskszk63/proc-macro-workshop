use proc_macro::TokenStream;

mod imp;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    imp::derive(input.into()).into()
}
