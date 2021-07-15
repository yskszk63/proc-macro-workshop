use proc_macro2::TokenStream;
use syn::{Field, Fields, FieldsNamed, ItemStruct, Type, Ident};
use quote::{format_ident, quote};

struct FieldWrapper<'a>(&'a Field);

impl<'a> FieldWrapper<'a> {
    fn from(field: &'a Field) -> Self {
        Self(field)
    }

    fn ty(&self) -> &'a Type {
        &self.0.ty
    }

    fn getter(&self) -> Ident {
        format_ident!("get_{}", self.0.ident.as_ref().unwrap())
    }

    fn setter(&self) -> Ident {
        format_ident!("set_{}", self.0.ident.as_ref().unwrap())
    }
}

fn gen_standard(input: &ItemStruct, fields: &FieldsNamed) -> syn::Result<TokenStream> {
    let attrs = &input.attrs;
    let vis = &input.vis;
    let ident = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&input.generics, "unsupported"));
    }

    let fields = fields.named.iter().map(FieldWrapper::from).collect::<Vec<_>>();
    let field_tys = fields.iter().map(FieldWrapper::ty).collect::<Vec<_>>();
    let field_seq = (0..fields.len()).into_iter().collect::<Vec<_>>();
    let getters = fields.iter().map(FieldWrapper::getter).collect::<Vec<_>>();
    let setters = fields.iter().map(FieldWrapper::setter).collect::<Vec<_>>();

    let len = fields.len();
    let mut offsets = vec![];
    let mut last = vec![];
    for ty in &field_tys {
        offsets.push(last.clone());
        last.push(*ty);
    }

    Ok(quote! {
        #(#attrs)*
        #[repr(C)]
        #vis struct #ident {
            data: [u8; (((#(<#field_tys as ::bitfield::Specifier>::BITS)+*) - 1) >> 3) + 1],
        }

        impl #ident {
            const OFFSET: [usize; #len] = [
                #( 0 #(+ <#offsets as ::bitfield::Specifier>::BITS)*,)*
            ];

            pub fn new() -> Self {
                Self {
                    data: Default::default(),
                }
            }

            #(
                pub fn #getters(&self) -> <#field_tys as ::bitfield::Specifier>::Item {
                    let off = Self::OFFSET[#field_seq];
                    <#field_tys as ::bitfield::Specifier>::get(off, &self.data[..])
                }
                pub fn #setters(&mut self, val: <#field_tys as ::bitfield::Specifier>::Item) {
                    let off = Self::OFFSET[#field_seq];
                    <#field_tys as ::bitfield::Specifier>::set(off, &mut self.data[..], val)
                }
            )*
        }

        impl ::bitfield::checks::TotalSizeModEight<{(0 #( + <#field_tys as ::bitfield::Specifier>::BITS )* ) % 8}> for #ident {}
        impl ::bitfield::checks::TotalSizeIsMultipleOfEightBits for #ident {}
    })
}

fn gen(input: ItemStruct) -> syn::Result<TokenStream> {
    match &input.fields {
        Fields::Named(fields) => gen_standard(&input, fields),
        _ => todo!(),
    }
}

pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = if let Ok(item) = syn::parse2::<ItemStruct>(input.clone()) {
        item
    } else {
        return syn::Error::new_spanned(args, "expect struct").to_compile_error();
    };

    match gen(item) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}
