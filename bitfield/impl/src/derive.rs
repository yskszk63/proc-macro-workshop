use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Expr, Ident, Type, Variant, parse_quote};
use syn::visit::{self, Visit};

struct BitsDetector<'b>(&'b mut Option<u64>, &'b mut Vec<syn::Error>);

impl<'ast, 'b> Visit<'ast> for BitsDetector<'b> {
    fn visit_lit_int(&mut self, i: &'ast syn::LitInt) {
        match (i.base10_parse::<u64>(), &self.0) {
            (Ok(v), None) => *self.0 = Some(v),
            (Ok(v), Some(v1)) => *self.0 = Some(v.max(*v1)),
            (Err(err), _) => self.1.push(err),
        }
        visit::visit_lit_int(self, i);
    }
}

fn detectbits(item: &DataEnum) -> syn::Result<usize> {
    let mut max = None;
    let mut errs = vec![];
    BitsDetector(&mut max, &mut errs).visit_data_enum(item);

    if !errs.is_empty() {
        return Err(errs.into_iter().next().unwrap());
    }
    let mut max = if let Some(max) = max {
        max
    } else {
        return Err(syn::Error::new_spanned(&item.variants, "no literal specified."));
    };

    let mut bits = 0;
    while max != 0 {
        bits += 1;
        max >>= 1;
    }
    Ok(bits.max(1))
}

fn ty_for_bits(b: usize) -> syn::Result<Type> {
    Ok(match b {
        0..=8 => parse_quote! { u8 },
        9..=16 => parse_quote! { u16 },
        17..=32 => parse_quote! { u32 },
        33..=64 => parse_quote! { u64 },
        x => return Err(syn::Error::new(Span::call_site(), format!("unsupported {} bits", x))),
    })
}

struct VariantWrapper<'b>(&'b Variant);

impl<'b> VariantWrapper<'b> {
    fn from(v: &'b Variant) -> syn::Result<Self> {
        if v.discriminant.is_none() {
            return Err(syn::Error::new_spanned(v, "needs discriminant"));
        }
        Ok(Self(v))
    }

    fn ident(&self) -> &'b Ident {
        &self.0.ident
    }

    fn val(&self) -> &'b Expr {
        &self.0.discriminant.as_ref().unwrap().1
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

    let variants = data.variants.iter().map(VariantWrapper::from).collect::<syn::Result<Vec<_>>>()?;
    let vidents = variants.iter().map(VariantWrapper::ident).collect::<Vec<_>>();
    let vvals = variants.iter().map(VariantWrapper::val).collect::<Vec<_>>();

    let ident = &input.ident;
    let bits = detectbits(&data)?;
    let ty = ty_for_bits(bits)?;

    Ok(quote! {
        impl ::bitfield::Specifier for #ident {
            const BITS: usize = #bits;
            type Item = #ty;
            type Type = Self;

            fn from(me: Self::Type) -> Self::Item {
                match me {
                    #(Self::#vidents => #vvals,)*
                }
            }

            fn to(they: Self::Item) -> Self::Type {
                match they {
                    #(#vvals => Self::#vidents,)*
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

