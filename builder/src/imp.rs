use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, GenericArgument, Ident, LitStr, Path, PathArguments, PathSegment, Token, Type, TypePath};
use syn::spanned::Spanned;
use syn::parse::{Parse, ParseStream};

#[derive(Default)]
struct Attrs {
    each: Option<LitStr>,
}

impl Attrs {
    fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        for attr in attrs {
            if attr.path.is_ident("builder") {
                let attr = match attr.parse_args::<Self>() {
                    Ok(attr) => attr,
                    Err(err) => return Err(syn::Error::new_spanned(attr.parse_meta()?, err)),
                };
                return Ok(attr); // TODO merge
            }
        }
        Ok(Default::default())
    }
}

impl Parse for Attrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut each = None;

        while input.peek(Ident) {
            let name = input.parse::<Ident>()?;
            if name == "each" {
                input.parse::<Token![=]>()?;
                each = Some(input.parse::<LitStr>()?);
            } else {
                return Err(syn::Error::new(input.span(), r#"expected `builder(each = "...")`"#));
            }
        }

        Ok(Self {
            each,
        })
    }
}

fn infer_option(ty: &Type) -> (bool, &Type) {
    if let Type::Path(TypePath { path: Path { segments, .. }, .. }) = ty {
        if segments.len() == 1 {
            let segment = segments.last().unwrap();
            if let PathSegment { ident, arguments: PathArguments::AngleBracketed(args) } = segment {
                if ident == "Option" && args.args.len() == 1 {
                    if let GenericArgument::Type(ty) = args.args.first().unwrap() {
                        return (true, ty);
                    }
                }
            }
        }
    }

    (false, ty)
}

struct TargetField<'a> {
    option: bool,
    ident: &'a Ident,
    ty: &'a Type,
    each: Option<Ident>,
}

impl<'a> TargetField<'a> {
    fn from(field: &'a syn::Field) -> syn::Result<Self> {
        let attrs = Attrs::from_attrs(&field.attrs)?;
        let (option, ty) = infer_option(&field.ty);

        Ok(Self {
            option,
            ident: field.ident.as_ref().unwrap(),
            ty,
            each: attrs.each.map(|f| format_ident!("{}", f.value())),
        })
    }

    fn standard(&self) -> bool {
        !self.option() && !self.each()
    }

    fn option(&self) -> bool {
        self.option
    }

    fn each(&self) -> bool {
        self.each.is_some()
    }

    fn standard_ident(&self) -> Option<&'a Ident> {
        self.standard().then(|| self.ident)
    }

    fn standard_ty(&self) -> Option<&'a Type> {
        self.standard().then(|| self.ty)
    }

    fn option_ident(&self) -> Option<&'a Ident> {
        self.option().then(|| self.ident)
    }

    fn option_ty(&self) -> Option<&'a Type> {
        self.option().then(|| self.ty)
    }

    fn each_ident<'s>(&'s self) -> Option<&'s Ident> {
        self.each().then(|| self.each.as_ref().unwrap())
    }

    fn each_ty(&self) -> Option<&'a Type> {
        self.each().then(|| self.ty)
    }

    fn each_owner(&self) -> Option<&'a Ident> {
        self.each().then(|| self.ident)
    }
}

fn builder(input: &DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let builder_ident = format_ident!("{}Builder", ident);

    let fields = if let DeriveInput { data: Data::Struct(data), .. } = input {
        data.fields.iter().map(TargetField::from).collect::<syn::Result<Vec<_>>>()?
    } else {
        return Err(syn::Error::new(input.span(), "enum or union not supported."));
    };

    let fidents = fields.iter().filter_map(TargetField::standard_ident).collect::<Vec<_>>();
    let ftys = fields.iter().filter_map(TargetField::standard_ty).collect::<Vec<_>>();

    let opt_fidents = fields.iter().filter_map(TargetField::option_ident).collect::<Vec<_>>();
    let opt_ftys = fields.iter().filter_map(TargetField::option_ty).collect::<Vec<_>>();

    let eachs = fields.iter().filter_map(TargetField::each_ident).collect::<Vec<_>>();
    let each_tys = fields.iter().filter_map(TargetField::each_ty).collect::<Vec<_>>();
    let each_owners = fields.iter().filter_map(TargetField::each_owner).collect::<Vec<_>>();

    Ok(quote! {
        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#fidents: None,)*
                    #(#opt_fidents: None,)*
                    #(#each_owners: ::std::default::Default::default(),)*
                }
            }
        }

        #[derive(Debug)]
        pub struct #builder_ident {
            #(#fidents: ::std::option::Option<#ftys>,)*
            #(#opt_fidents: ::std::option::Option<#opt_ftys>,)*
            #(#each_owners: #each_tys,)*
        }

        impl #builder_ident {
            #(
                pub fn #fidents(&mut self, val: #ftys) -> &mut Self {
                    self.#fidents = Some(val);
                    self
                }
            )*
                #(
                    pub fn #opt_fidents(&mut self, val: #opt_ftys) -> &mut Self {
                        self.#opt_fidents = Some(val);
                        self
                    }
                )*
                #(
                    pub fn #eachs<T>(&mut self, val: T) -> &mut Self where #each_tys: ::std::iter::Extend<T> {
                        self.#each_owners.extend([val]);
                        self
                    }
                )*

                pub fn build(&mut self) -> ::std::option::Option<#ident> {
                    #(
                        let #fidents = if let Some(val) = self.#fidents.clone() {
                            val
                        } else {
                            return None;
                        };
                    )*
                        #(let #opt_fidents = self.#opt_fidents.clone();)*
                        #(let #each_owners = self.#each_owners.clone();)*

                        Some(#ident {
                            #(#fidents,)*
                            #(#opt_fidents,)*
                            #(#each_owners,)*
                        })
                }
        }
    })
}

pub fn derive(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<DeriveInput>(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error(),
    };
    let builder = match builder(&input) {
        Ok(builder) => builder,
        Err(err) => return err.to_compile_error(),
    };

    quote! {
        #builder
    }
}

