#![coverage(off)]
#![feature(coverage_attribute)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, Attribute, Error, Expr, Item, ItemFn, ItemMod};

/// Attribute macro #[rust_main]
/// Generates a function named `main` that calls the user's original `main` function.
/// Allowing the same entry for both baremetal and std executables.
#[proc_macro_attribute]
pub fn rust_main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse the annotated item as a function
    let input_fn = parse_macro_input!(item as ItemFn);
    let sig = &input_fn.sig;

    // Basic checks: no generics, not async, not const
    if !sig.generics.params.is_empty() {
        return Error::new_spanned(
            sig.generics.clone(),
            "rust_main: generic functions are not supported",
        )
        .to_compile_error()
        .into();
    }
    if sig.asyncness.is_some() {
        return Error::new_spanned(
            sig.asyncness,
            "rust_main: async functions are not supported (wrap in a sync function)",
        )
        .to_compile_error()
        .into();
    }
    if sig.constness.is_some() {
        return Error::new_spanned(sig.constness, "rust_main: main function cannot be const")
            .to_compile_error()
            .into();
    }

    // Requires function name to be `main`
    // TODO: should we allow other names?
    if sig.ident != "main" {
        return Error::new_spanned(
            sig.ident.clone(),
            "rust_main: function must be named 'main'",
        )
        .to_compile_error()
        .into();
    }

    let impl_ident = format_ident!("__rust_main_impl"); // Transformed function

    // take the original function but rename it to impl_ident and make it private
    // keep its visibility as-is but we will declare it non-mangled regular fn
    let vis = &input_fn.vis;
    let attrs = &input_fn.attrs;
    let block = &input_fn.block;
    let inputs = &input_fn.sig.inputs;
    let output = &input_fn.sig.output;

    // Require no arguments for simplicity; if you need arguments (argc/argv),
    // you can extend this macro to accept them and pass via runtime.
    if !inputs.is_empty() {
        return Error::new_spanned(
            inputs.clone(),
            "rust_main: function must have no parameters",
        )
        .to_compile_error()
        .into();
    }

    // Resolve the `runtime` crate path (handles dependency renames).
    let runtime_path: syn::Path = match crate_name("runtime") {
        Ok(FoundCrate::Itself) => syn::parse_quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, Span::call_site());
            syn::parse_quote!(#ident)
        }
        Err(_) => syn::parse_quote!(runtime), // fallback
    };

    // Compose generated tokens:
    // 1) the renamed function with original body
    // 2) generated real main that calls runtime::rust_load_main
    let expanded = quote! {
        // keep user's attributes (except our attribute) on the implementation
        #(#attrs)*
        #vis fn #impl_ident() #output {
            #block
        }

        #[doc(hidden)]
        #[doc = "Generated entry point for rust standalone executable"]
        #[deprecated = concat!("Use `", stringify!(#impl_ident), "` instead")]
        #[inline(always)] // try to inline this function to `rust_main_entry`
        fn main() #output {
            // Delegate to runtime and return its value.
            #runtime_path::rust_load_main(#impl_ident)
        }

        #[doc(hidden)]
        mod __runtime_generated {
            /// generated entry for runtime crate to load the user's main
            #[unsafe(no_mangle)]
            extern "Rust" fn rust_main_entry() {
                #[allow(deprecated)]
                let _ = super::main();
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn ktest(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as Item);

    match input {
        Item::Fn(func) => expand_fn(func).into(),
        Item::Mod(module) => expand_mod(module).into(),
        other => syn::Error::new_spanned(other, "#[ktest] can only be applied to fn or mod")
            .to_compile_error()
            .into(),
    }
}

enum ExpectationSymbol {
    Success,
    ShouldPanic,
    ShouldPanicWithMessage(String),
}

fn parse_test_expectation(attrs: &[Attribute]) -> syn::Result<ExpectationSymbol> {
    for attr in attrs {
        if attr.path().is_ident("should_panic") {
            // Parse the attribute tokens to extract the expectation
            match &attr.meta {
                // Simple #[should_panic]
                syn::Meta::Path(_) => {
                    return Ok(ExpectationSymbol::ShouldPanic);
                }
                // #[should_panic = "message"]
                syn::Meta::NameValue(nv) => {
                    if let Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) = &nv.value
                    {
                        return Ok(ExpectationSymbol::ShouldPanicWithMessage(lit_str.value()));
                    } else {
                        return Err(syn::Error::new_spanned(
                            &nv.value,
                            "expected value must be a string",
                        ));
                    }
                }
                // #[should_panic(expected = "message")]
                syn::Meta::List(ml) => {
                    let mut message = None;

                    ml.parse_nested_meta(|l| {
                        if l.path.is_ident("expected") {
                            l.value()?.parse::<syn::LitStr>().map(|s| {
                                message = Some(s.value());
                            })
                        } else {
                            Err(l.error("expected `expected`"))
                        }
                    })?;

                    if let Some(msg) = message {
                        return Ok(ExpectationSymbol::ShouldPanicWithMessage(msg));
                    } else {
                        return Err(syn::Error::new_spanned(ml, "expected `message` argument"));
                    }
                }
            }
        }
    }
    Ok(ExpectationSymbol::Success)
}

fn expand_fn(func: ItemFn) -> proc_macro2::TokenStream {
    let span = func.span();

    let attrs = func.attrs;
    let vis = func.vis;
    let sig = func.sig;
    let block = func.block;
    let ident = sig.ident;

    let test_desc = format_ident!("test_desc_{}", ident,);

    let (start_line, start_col) = (span.start().line, span.start().column);
    let (end_line, end_col) = (span.end().line, span.end().column);

    // Resolve the `runtime` crate path (handles dependency renames).
    let runtime_path: syn::Path = match crate_name("runtime") {
        Ok(FoundCrate::Itself) => syn::parse_quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, Span::call_site());
            syn::parse_quote!(#ident)
        }
        Err(_) => syn::parse_quote!(runtime), // fallback
    };

    let expectation = match parse_test_expectation(&attrs) {
        Ok(opt) => opt,
        Err(e) => return e.to_compile_error(),
    };

    let expect = match expectation {
        ExpectationSymbol::Success => quote! { #runtime_path::test::ResultExpectation::Success },
        ExpectationSymbol::ShouldPanic => {
            quote! { #runtime_path::test::ResultExpectation::ShouldPanic }
        }
        ExpectationSymbol::ShouldPanicWithMessage(msg) => {
            let msg = msg.as_str();
            quote! { #runtime_path::test::ResultExpectation::ShouldPanicWithMessage(#msg) }
        }
    };

    quote! {
        #[doc(hidden)]
        const _: () = {
            #[used]
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            #[cfg_attr(target_os = "none", link_section = ".ktest_array")]
            static #test_desc: #runtime_path::test::TestDesc = #runtime_path::test::TestDesc {
                name: ::core::prelude::v1::stringify!(#ident),
                module_path: ::core::prelude::v1::module_path!(),
                package: ::core::prelude::v1::env!("CARGO_PKG_NAME"),
                source_file: ::core::prelude::v1::file!(),
                expect: #expect,
                start: #runtime_path::test::SourcePosition {
                    line: #start_line,
                    column: #start_col,
                },
                end: #runtime_path::test::SourcePosition {
                    line: #end_line,
                    column: #end_col,
                },
                func: #ident,
            };
        };

        #[cfg_attr(not(target_os = "none"), ::core::prelude::v1::test)]
        #(#attrs)*
        #vis fn #ident() #block
    }
}

fn expand_mod(module: ItemMod) -> proc_macro2::TokenStream {
    let attrs = module.attrs;
    let vis = module.vis;
    let ident = module.ident;
    let content = module.content;

    let attrs = quote! {
        #[cfg(any(test, ktest))]
        #(#attrs)*
    };

    if let Some((_, items)) = content {
        quote! {
            #attrs
            #vis mod #ident {
                #(#items)*
            }
        }
    } else {
        quote! {
            #attrs
            #vis mod #ident;
        }
    }
}
