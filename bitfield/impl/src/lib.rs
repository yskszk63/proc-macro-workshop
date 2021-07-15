use proc_macro::TokenStream;

mod imp;

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    imp::bitfield(args.into(), input.into()).into()
}
