use std::mem;
use std::cmp;

use proc_macro2::TokenStream;
use syn::{ExprMatch, ItemEnum, ItemFn, Pat, PatTupleStruct, PatStruct, PatPath};
use syn::visit_mut::{self, VisitMut};
use syn::spanned::Spanned;
use quote::ToTokens;

fn sorted_enum(input: &ItemEnum) -> syn::Result<()> {
    let variants = input.variants.iter().collect::<Vec<_>>();
    let mut sorted = variants.clone();
    sorted.sort_by(|l, r| l.ident.cmp(&r.ident));

    let mut iter = variants.iter().zip(sorted.iter()).peekable();
    while let Some((l, r)) = iter.next() {
        if l != r {
            if let Some((_, next)) = iter.peek() {
                return Err(syn::Error::new_spanned(&r.ident, format!("{} should sort before {}", r.ident, next.ident)));
            }
        }
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
struct PatWrapper<'a>(&'a Pat);

impl<'a> PatWrapper<'a> {
    fn try_from(p: &'a Pat) -> syn::Result<Self> {
        match p {
            Pat::Path(PatPath { path, .. }) | Pat::Struct(PatStruct { path, .. }) | Pat::TupleStruct(PatTupleStruct { path, .. }) if path.segments.last().is_some() => Ok(Self(p)),
            Pat::Ident(..) | Pat::Wild(..) => Ok(Self(p)),
            _ => Err(syn::Error::new_spanned(p, "unsupported by #[sorted]")),
        }
    }

    fn ident(&self) -> String {
        match self.0 {
            Pat::Path(PatPath { path, .. }) | Pat::Struct(PatStruct { path, .. }) | Pat::TupleStruct(PatTupleStruct { path, .. }) => path.to_token_stream().to_string().replace(" ", ""), // FIXME
            Pat::Ident(item) => item.ident.to_string(),
            Pat::Wild(..) => "_".into(),
            _ => unreachable!("{:?}", self.0),
        }
    }
}

impl Spanned for PatWrapper<'_> {
    fn span(&self) -> proc_macro2::Span {
        match self.0 {
            Pat::Path(PatPath { path, .. }) | Pat::Struct(PatStruct { path, .. }) | Pat::TupleStruct(PatTupleStruct { path, .. }) => path.span(),
            Pat::Ident(item) => item.span(),
            Pat::Wild(item) => item.span(),
            _ => unreachable!("{:?}", self.0),
        }
    }
}

impl cmp::PartialOrd for PatWrapper<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for PatWrapper<'_> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match (self.0, other.0) {
            (Pat::Wild(..), Pat::Wild(..)) => cmp::Ordering::Equal,
            (Pat::Wild(..), _) => cmp::Ordering::Greater,
            (_, Pat::Wild(..)) => cmp::Ordering::Less,
            (_, _) => self.ident().cmp(&other.ident()),
        }
    }
}

fn sorted_match(input: &ExprMatch) -> syn::Result<()> {
    let arms = input.arms.iter().map(|a| PatWrapper::try_from(&a.pat)).collect::<syn::Result<Vec<_>>>()?;
    let mut sorted = arms.clone();
    sorted.sort();

    let mut iter = arms.iter().zip(sorted.iter()).peekable();
    while let Some((l, r)) = iter.next() {
        if l != r {
            if let Some((_, next)) = iter.peek() {
                return Err(syn::Error::new(r.span(), format!("{} should sort before {}", r.ident(), next.ident())));
            }
        }
    }
    Ok(())
}

fn try_sorted(attr: TokenStream, input: TokenStream) -> syn::Result<()> {
    match syn::parse2::<ItemEnum>(input.clone()) {
        Ok(item) => {
            sorted_enum(&item)?;
            return Ok(());
        }
        Err(..) => {}
    };
    match syn::parse2::<ExprMatch>(input.clone()) {
        Ok(item) => {
            sorted_match(&item)?;
            return Ok(());
        }
        Err(..) => Err(syn::Error::new_spanned(attr, "expected enum or match expression"))
    }
}

pub fn sorted(attr: TokenStream, input: TokenStream) -> TokenStream {
    if let Err(err) = try_sorted(attr, input.clone()) {
        err.to_compile_error().into_iter().chain(input).collect()
    } else {
         input
    }
}

struct SortedVisitor<'a>(&'a mut Vec<syn::Error>);

impl<'a> VisitMut for SortedVisitor<'a> {
    fn visit_expr_match_mut(&mut self, i: &mut ExprMatch) {
        let mut found = None;
        let mut newattrs = vec![];
        for attr in &mut i.attrs.drain(..) {
            if attr.path.is_ident("sorted") {
                found = Some(attr);
            } else {
                newattrs.push(attr);
            }
        }
        mem::swap(&mut newattrs, &mut i.attrs);

        if let Some(attr) = found {
            if let Err(err) = try_sorted(attr.into_token_stream(), i.into_token_stream()) {
                self.0.push(err);
            }
        }

        visit_mut::visit_expr_match_mut(self, i)
    }
}


pub fn check(_: TokenStream, input: TokenStream) -> TokenStream {
    match syn::parse2::<ItemFn>(input) {
        Ok(mut item) => {
            let mut errors = vec![];
            SortedVisitor(&mut errors).visit_item_fn_mut(&mut item);
            errors.into_iter().flat_map(|err| err.to_compile_error().into_iter()).chain(item.to_token_stream()).collect()
        }
        Err(err) => err.to_compile_error(),
    }
}
