extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::{Ident, Literal};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, parse_quote, ItemFn, Token};

mod kw {
    syn::custom_keyword!(cached);
}

enum CacheName {
    Explicit(Ident),
    Implicit,
}

struct Args {
    cached: Option<CacheName>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(kw::cached) {
            input.parse::<kw::cached>()?;
            if input.peek(Token![=]) {
                input.parse::<Token![=]>()?;
                let ident = input.parse::<Ident>()?;
                Ok(Self {
                    cached: Some(CacheName::Explicit(ident)),
                })
            } else {
                Ok(Self {
                    cached: Some(CacheName::Implicit),
                })
            }
        } else {
            Ok(Self { cached: None })
        }
    }
}

struct DocExtractor {
    _eq: Token![=],
    doc: Literal,
}

impl Parse for DocExtractor {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _eq: input.parse()?,
            doc: input.parse()?,
        })
    }
}

#[allow(clippy::missing_panics_doc, clippy::too_many_lines)]
#[proc_macro_attribute]
pub fn helper_func(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let input = parse_macro_input!(input as ItemFn);

    let vis = &input.vis;
    let attrs = &input.attrs;
    let arg_names: Vec<_> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(pat_type) => match &*pat_type.pat {
                syn::Pat::Ident(pat_ident) => Some(&pat_ident.ident),
                _ => None,
            },
        })
        .collect();
    let block = &input.block;

    let old_func_ident = Ident::new(&format!("__{}", input.sig.ident), input.sig.ident.span());
    let old_func = {
        let mut sig = input.sig.clone();
        sig.ident = old_func_ident.clone();
        quote! {
            #(#attrs)*
            #vis #sig {
                #block
            }
        }
    };

    let registry = {
        let util_ident = Ident::new(
            &format!("__UTIL_{}", input.sig.ident),
            input.sig.ident.span(),
        );
        let func_doc = attrs
            .iter()
            .filter(|a| a.path.is_ident("doc"))
            .map(|a| {
                let doc: DocExtractor = syn::parse2(a.tokens.clone()).unwrap();
                doc.doc
            })
            .next()
            .unwrap_or_else(|| Literal::string(""));
        let func_sig = {
            let func_name = &input.sig.ident;
            quote! {
                #func_name(#(#arg_names),*)
            }
        }
        .to_string();
        let func_ident = &input.sig.ident;
        quote! {
            #[allow(non_upper_case_globals)]
            #[linkme::distributed_slice(crate::UTILS)]
            static #util_ident: (&str, &str, once_cell::sync::Lazy<minijinja::value::Value>) = (
                #func_sig,
                #func_doc,
                once_cell::sync::Lazy::new(|| {
                    minijinja::value::Value::from_function(#func_ident)
                })
            );
        }
    };

    let allow_attrs = quote! {
        #[allow(clippy::used_underscore_binding)]
    };
    let derived_func = {
        let mut sig = input.sig.clone();
        sig.output = parse_quote!( -> Result<String, minijinja::Error> );
        match args.cached {
            None => quote! {
                #allow_attrs
                #vis #sig {
                    let value = #old_func_ident(#(#arg_names),*);
                    value.map_err(|e| {
                        minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, e.to_string())
                    })
                }
            },
            Some(cache_name) => {
                let cache_key = match cache_name {
                    CacheName::Explicit(ident) => ident.to_string(),
                    CacheName::Implicit => sig.ident.to_string(),
                };
                quote! {
                    #allow_attrs
                    #vis #sig {
                        let store = crate::store::get_global_store();
                        let path = &[#cache_key.to_string(), #((&#arg_names).to_string()),*];
                        if let Some(cache) = store.try_get_cached(path) {
                            Ok(cache)
                        } else {
                            let value = #old_func_ident(#(#arg_names),*);
                            let value = value.map_err(|e| {
                                minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, e.to_string())
                            });
                            if let Ok(ref value) = value {
                                store.put_cache(path, value.to_string());
                            }
                            value
                        }
                    }
                }
            }
        }
    };

    let tokens = quote! {
        #old_func
        #registry
        #derived_func
    };

    tokens.into()
}
