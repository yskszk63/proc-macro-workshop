use std::ops;
use std::mem;

use proc_macro2::{Delimiter, Group, Literal, TokenStream, TokenTree};
use quote::format_ident;
use syn::{Ident, LitInt, Token};
use syn::parse::{Parse, ParseStream};

#[derive(Clone)]
enum Range {
    Exclusive(ops::Range<usize>),
    Inclusive(ops::RangeInclusive<usize>),
}

impl Iterator for Range {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Inclusive(iter) => iter.next(),
            Self::Exclusive(iter) => iter.next(),
        }
    }
}

struct Input {
    ident: Ident,
    range: Range,
    tokens: TokenStream,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        input.parse::<Token![in]>()?;
        let begin = input.parse::<LitInt>()?;
        let range = if input.peek(Token![..=]) {
            input.parse::<Token![..=]>()?;
            let end = input.parse::<LitInt>()?;
            Range::Inclusive(begin.base10_parse()?..=end.base10_parse()?)
        } else {
            input.parse::<Token![..]>()?;
            let end = input.parse::<LitInt>()?;
            Range::Exclusive(begin.base10_parse()?..end.base10_parse()?)
        };
        let group = input.parse::<Group>()?;
        let tokens = group.stream();
        Ok(Self {
            ident,
            range,
            tokens,
        })
    }
}

fn expand(ident: &Ident, n: usize, tokens: TokenStream) -> syn::Result<TokenStream> {
    let mut result = vec![];

    let mut tokens = tokens.into_iter().peekable();
    let mut back = None;

    while let Some(mut tree) = tokens.next() {
        match &tree {
            TokenTree::Group(g) => {
                tree = Group::new(g.delimiter(), expand(ident, n, g.stream())?).into();
            }

            TokenTree::Ident(i) => {
                if ident == i {
                    tree = Literal::usize_unsuffixed(n).into();
                }
            }

            TokenTree::Punct(p) => {
                if let (Some(TokenTree::Ident(backref)), '#', Some(TokenTree::Ident(next))) = (&back, p.as_char(), tokens.peek()) {
                    if ident == next {
                        tokens.next();

                        let newtoken = format_ident!("{}{}", backref, n);
                        back = Some(newtoken.into());
                        continue;
                    }
                }
            }

            _ => {}
        }

        if let Some(back) = &mut back {
            mem::swap(back, &mut tree);
            result.push(tree);
        } else {
            back = Some(tree);
        }
    }

    if let Some(back) = back {
        result.push(back);
    }
    Ok(result.into_iter().collect())
}

fn traverse(expanded: &mut bool, ident: &Ident, range: Range, tokens: TokenStream) -> syn::Result<TokenStream> {
    let mut tokens = tokens.into_iter().peekable();

    let mut result = vec![];
    let mut back = None;

    while let Some(mut tree) = tokens.next() {
        match tree {
            TokenTree::Group(g) => {
                if let (Some(TokenTree::Punct(b)), Delimiter::Parenthesis, Some(TokenTree::Punct(n))) = (back.as_ref(), g.delimiter(), tokens.peek()) {
                    if b.as_char() == '#' && n.as_char() == '*' {
                        tokens.next();
                        back = None;

                        for n in range.clone() {
                            let e = expand(ident, n, g.stream())?;
                            result.extend(e);
                        }
                        *expanded = true;
                        continue;
                    }
                }
                tree = Group::new(g.delimiter(), traverse(expanded, ident, range.clone(), g.stream())?).into();
            }
            _ => {}
        }

        if let Some(back) = &mut back {
            mem::swap(back, &mut tree);
            result.push(tree);
        } else {
            back = Some(tree);
        }
    }

    if let Some(back) = back {
        result.push(back);
    }

    Ok(result.into_iter().collect())
}

pub fn seq(input: TokenStream) -> TokenStream {
    let Input { ident, range, tokens } = match syn::parse2(input) {
        Ok(item) => item,
        Err(err) => return err.to_compile_error(),
    };

    let mut expanded = false;
    let body = match traverse(&mut expanded, &ident, range.clone(), tokens.clone()) {
        Ok(body) => body,
        Err(err) => return err.to_compile_error(),
    };
    if expanded {
        return body;
    }

    let mut bodies = vec![];
    for n in range {
        let body = match expand(&ident, n, tokens.clone()) {
            Ok(body) => body,
            Err(err) => return err.to_compile_error(),
        };
        bodies.push(body);
    }

    bodies.into_iter().collect()
}
