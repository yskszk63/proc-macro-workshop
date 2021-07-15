use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Ident, Variant};

struct VariantWrapper<'b>(&'b Variant);

impl<'b> VariantWrapper<'b> {
    fn ident(&self) -> &'b Ident {
        &self.0.ident
    }
}

fn gen(input: DeriveInput) -> syn::Result<TokenStream> {
    let data = match input.data {
        Data::Enum(item) => item,
        _ => return Err(syn::Error::new_spanned(input, "not supported.")),
    };

    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(input.generics, "generics not supported."));
    }

    if data.variants.len().count_ones() != 1 {
        return Err(syn::Error::new(Span::call_site(), "BitfieldSpecifier expected a number of variants which is a power of 2"));
    }

    let variants = data.variants.iter().map(VariantWrapper).collect::<Vec<_>>();
    let vidents = variants.iter().map(VariantWrapper::ident).collect::<Vec<_>>();

    let ident = &input.ident;

    Ok(quote! {
        impl ::bitfield::Specifier for #ident {
            const BITS: usize = {
                let bits = 0u64 #( | Self::#vidents as u64 )*;
                (64 - bits.leading_zeros()) as usize
            };
            type Type = Self;

            fn to(me: Self::Type) -> u64 {
                me as u64
            }

            fn from(they: u64) -> Self::Type {
                #![allow(non_upper_case_globals)]
                #( const #vidents: u64 = #ident::#vidents as u64;)*
                match they {
                    #(#vidents => Self::#vidents,)*
                    _ => panic!(),
                }
            }
        }
    })
}

pub fn derive(input: TokenStream) -> TokenStream {
    match syn::parse2::<DeriveInput>(input).and_then(gen) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

