use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Field, GenericArgument, GenericParam, Generics, Ident, LitStr, Path, PathArguments, PathSegment, Token, Type, TypePath, parse_quote};
use syn::parse::{Parse, ParseStream};

struct DebugAttr(String);

impl Default for DebugAttr {
    fn default() -> Self {
        Self("{:?}".into())
    }
}

impl DebugAttr {
    fn from_field(field: &Field) -> syn::Result<Self> {
        for attr in &field.attrs {
            if attr.path.is_ident("debug") {
                return Ok(syn::parse2(attr.tokens.clone())?);
            }
        }
        Ok(Default::default())
    }
}

impl Parse for DebugAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![=]>()?;
        let val = input.parse::<LitStr>()?.value();
        Ok(Self(val))
    }
}

struct CollectPhantomDataT<'a, 'b>(&'b mut Vec<&'a GenericArgument>);

fn infer_phantomdata_t<'a>(ty: &'a Type) -> Option<&'a Ident> {
    if let Type::Path(TypePath { path: Path { segments, .. }, ..}) = ty {
        if let Some(PathSegment { ident, arguments: PathArguments::AngleBracketed(args), }) = segments.last() {
            if ident == "PhantomData" && args.args.len() == 1 {
                if let Some(GenericArgument::Type(t)) = args.args.first() {
                    if let Type::Path(TypePath { path: Path { segments, .. }, .. }) = t {
                        if let Some(segment) = segments.first() {
                            return Some(&segment.ident);
                        }
                    }
                }
            }
        }
    }
    None
}

struct TargetField<'a> {
    ident: &'a Ident,
    debug: String,
    phantomdata_t: Option<&'a Ident>,
}

impl<'a> TargetField<'a> {
    fn from(field: &'a Field) -> syn::Result<Self> {
        let ident = field.ident.as_ref().unwrap();
        let DebugAttr(debug) = DebugAttr::from_field(field)?;
        let phantomdata_t = infer_phantomdata_t(&field.ty);
        Ok(Self {
            ident,
            debug,
            phantomdata_t,
        })
    }

    fn ident_str(&self) -> String {
        self.ident.to_string()
    }
}

fn add_trait_bounds(generics: &Generics, ignores: &Vec<&Ident>) -> Generics {
    let mut generics = generics.clone();
    for param in &mut generics.params {
        if let GenericParam::Type(param) = param {
            if !ignores.contains(&&param.ident) {
                param.bounds.push(parse_quote! { ::std::fmt::Debug });
            }
        }
    }
    generics
}

fn debug(input: &DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let ident_str = ident.to_string();

    let fields = if let DeriveInput { data: Data::Struct(data), .. } = input {
        data.fields.iter().map(TargetField::from).collect::<syn::Result<Vec<_>>>()?
    } else {
        return Err(syn::Error::new_spanned(input, "enum or union not supported."));
    };

    let field_names = fields.iter().map(|f| f.ident).collect::<Vec<_>>();
    let field_strs = fields.iter().map(TargetField::ident_str).collect::<Vec<_>>();
    let debugs = fields.iter().map(|f| &f.debug).collect::<Vec<_>>();
    let phantom_ts = fields.iter().filter_map(|f| f.phantomdata_t).collect::<Vec<_>>();

    let generics = add_trait_bounds(&input.generics, &phantom_ts);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::std::fmt::Debug for #ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.debug_struct(#ident_str)
                    #(.field(#field_strs, &format_args!(#debugs, &self.#field_names)))*
                    .finish()
            }
        }
    })
}

pub fn derive(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<DeriveInput>(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error(),
    };
    match debug(&input) {
        Ok(token) => token,
        Err(err) => return err.to_compile_error(),
    }
}
