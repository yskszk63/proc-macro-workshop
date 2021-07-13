use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Field, GenericArgument, Ident, LitStr, Token, Type, WhereClause, WherePredicate, parse_quote};
use syn::parse::{Parse, ParseStream};
use syn::visit::{self, Visit};

#[derive(Default)]
struct DebugAttr {
    bound: Option<LitStr>
}

impl DebugAttr {
    fn from_derive_input(input: &DeriveInput) -> syn::Result<Self> {
        for attr in &input.attrs {
            if attr.path.is_ident("debug") {
                return attr.parse_args();
            }
        }
        Ok(Default::default())
    }
}

impl Parse for DebugAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut bound = None;

        while input.peek(Ident) {
            let name = input.parse::<Ident>()?;
            if name == "bound" {
                input.parse::<Token![=]>()?;
                bound = input.parse()?;
            }
        }

        Ok(Self {
            bound,
        })
    }
}

struct FieldDebugAttr(String);

impl Default for FieldDebugAttr {
    fn default() -> Self {
        Self("{:?}".into())
    }
}

impl FieldDebugAttr {
    fn from_field(field: &Field) -> syn::Result<Self> {
        for attr in &field.attrs {
            if attr.path.is_ident("debug") {
                return Ok(syn::parse2(attr.tokens.clone())?);
            }
        }
        Ok(Default::default())
    }
}

impl Parse for FieldDebugAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![=]>()?;
        let val = input.parse::<LitStr>()?.value();
        Ok(Self(val))
    }
}

struct IsPhantomData<'ast, 'b>(&'b mut Option<&'ast Type>);

impl<'ast, 'b> Visit<'ast> for IsPhantomData<'ast, 'b> {
    fn visit_path(&mut self, i: &'ast syn::Path) {
        if let Some(last) = i.segments.last() {
            if last.ident == "PhantomData" {
                visit::visit_path_segment(self, last)
            }
        }
    }
    fn visit_generic_argument(&mut self, i: &'ast syn::GenericArgument) {
        if let GenericArgument::Type(ty) = i {
            *self.0 = Some(ty);
        }
    }
}

struct CollectPhantomDataT<'a, 'b>(&'b mut Vec<&'a Type>, bool);

impl<'a, 'b> Visit<'a> for CollectPhantomDataT<'a, 'b> {
    fn visit_type(&mut self, i: &'a Type) {
        let mut phantomdatat = None;
        IsPhantomData(&mut phantomdatat).visit_type(i);
        if let Some(ty) = phantomdatat {
            self.0.push(ty);
        }
    }
}

struct HasGenericArgument<'b>(&'b mut bool);

impl<'ast, 'b> Visit<'ast> for HasGenericArgument<'b> {
    fn visit_generic_argument(&mut self, _: &'ast syn::GenericArgument) {
        *self.0 = true
    }
}

struct CollectFieldTypes<'ast, 'b>(&'b mut Vec<&'ast Type>, Vec<&'ast Type>);

impl<'ast, 'b> Visit<'ast> for CollectFieldTypes<'ast, 'b> {
    fn visit_type(&mut self, i: &'ast Type) {
        let mut has_genric_argument = false;
        HasGenericArgument(&mut has_genric_argument).visit_type(i);
        if !has_genric_argument && !self.1.contains(&&i) {
            self.0.push(i);
        }
        visit::visit_type(self, i);
    }
}

struct TargetField<'a> {
    ident: &'a Ident,
    debug: String,
}

impl<'a> TargetField<'a> {
    fn from(field: &'a Field) -> syn::Result<Self> {
        let ident = field.ident.as_ref().unwrap();
        let FieldDebugAttr(debug) = FieldDebugAttr::from_field(field)?;
        Ok(Self {
            ident,
            debug,
        })
    }

    fn ident_str(&self) -> String {
        self.ident.to_string()
    }
}

fn debug(input: &DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let ident_str = ident.to_string();

    let fields = if let DeriveInput { data: Data::Struct(data), .. } = input {
        data.fields.iter().map(TargetField::from).collect::<syn::Result<Vec<_>>>()?
    } else {
        return Err(syn::Error::new_spanned(input, "enum or union not supported."));
    };

    let where_clause = if let DebugAttr { bound: Some(bound) } = DebugAttr::from_derive_input(input)? {
        let predicate = syn::parse_str::<WherePredicate>(&bound.value())?;
        Some(parse_quote! {
            where #predicate
        })
    } else {
        let mut phantom_ts = vec![];
        CollectPhantomDataT(&mut phantom_ts, false).visit_derive_input(input);
        let mut generic_types = vec![];
        CollectFieldTypes(&mut generic_types, phantom_ts).visit_derive_input(input);
        let where_clause = generic_types.into_iter().map(|g| {
            parse_quote! { #g: ::std::fmt::Debug }
        }).collect::<Vec<WherePredicate>>();
        if !where_clause.is_empty() {
            Option::<WhereClause>::Some(parse_quote! { where #(#where_clause),* })
        } else {
            None
        }
    };

    let field_names = fields.iter().map(|f| f.ident).collect::<Vec<_>>();
    let field_strs = fields.iter().map(TargetField::ident_str).collect::<Vec<_>>();
    let debugs = fields.iter().map(|f| &f.debug).collect::<Vec<_>>();

    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

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
