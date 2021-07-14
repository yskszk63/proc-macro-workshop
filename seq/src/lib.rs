use proc_macro::TokenStream;

mod imp;

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    imp::seq(input.into()).into()
}
