use proc_macro::TokenStream;

mod imp;
mod derive;

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    imp::bitfield(args.into(), input.into()).into()
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn derive(input: TokenStream) -> TokenStream {
    derive::derive(input.into()).into()
}
