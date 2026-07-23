//! `moonlit_plugin!` — generates the WIT `Guest` glue and `export!` for a
//! plugin from a declaration of its name, middlewares, and optional config/state.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, bracketed, Ident, LitStr, Token, Type};

/// `moonlit_plugin! { name: "git", config: C, middlewares: [A, B], state: S }`
struct PluginDecl {
    name: LitStr,
    config: Option<Type>,
    middlewares: Vec<Type>,
    state: Option<Type>,
}

impl Parse for PluginDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut config: Option<Type> = None;
        let mut middlewares: Option<Vec<Type>> = None;
        let mut state: Option<Type> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "name" => name = Some(input.parse()?),
                "config" => config = Some(input.parse()?),
                "state" => state = Some(input.parse()?),
                "middlewares" => {
                    let content;
                    bracketed!(content in input);
                    let types = content.parse_terminated(Type::parse, Token![,])?;
                    middlewares = Some(types.into_iter().collect());
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown moonlit_plugin! field `{other}` (expected name, config, middlewares, state)"),
                    ));
                }
            }
            // optional trailing comma between fields
            let _ = input.parse::<Token![,]>();
        }

        Ok(PluginDecl {
            name: name.ok_or_else(|| input.error("missing `name:`"))?,
            config,
            middlewares: middlewares.ok_or_else(|| input.error("missing `middlewares:`"))?,
            state,
        })
    }
}

#[proc_macro]
pub fn moonlit_plugin(input: TokenStream) -> TokenStream {
    // Allow trailing braces form `moonlit_plugin! { ... }`.
    let decl = syn::parse_macro_input!(input as PluginDeclInput).0;
    expand(decl).into()
}

/// Wrapper so the macro accepts either `{ ... }` or bare `...`.
struct PluginDeclInput(PluginDecl);
impl Parse for PluginDeclInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            Ok(PluginDeclInput(content.parse()?))
        } else {
            Ok(PluginDeclInput(input.parse()?))
        }
    }
}

fn expand(decl: PluginDecl) -> proc_macro2::TokenStream {
    let name = &decl.name;
    let mws = &decl.middlewares;

    // list-middlewares entries
    let list_entries = mws.iter().map(|m| {
        quote! {
            ::moonlit_plugin_sdk::bindings::MiddlewareInfo {
                name: <#m as ::moonlit_plugin_sdk::Middleware>::NAME.to_string(),
                description: <#m as ::moonlit_plugin_sdk::Middleware>::DESCRIPTION.to_string(),
            }
        }
    });

    // execute dispatch arms
    let exec_arms = mws.iter().map(|m| {
        quote! {
            <#m as ::moonlit_plugin_sdk::Middleware>::NAME => {
                let cfg: <#m as ::moonlit_plugin_sdk::Middleware>::Config =
                    match ::moonlit_plugin_sdk::config::from_json_value(&config) {
                        Ok(c) => c,
                        Err(e) => {
                            return ::moonlit_plugin_sdk::MiddlewareResult::failure(
                                ::std::format!("invalid config for `{}`: {}", middleware, e)
                            ).into_wit();
                        }
                    };
                let mw = <#m as ::core::default::Default>::default();
                ::moonlit_plugin_sdk::Middleware::execute(&mw, &ctx, cfg).into_wit()
            }
        }
    });

    // optional state static + install
    let (state_static, state_attach) = match &decl.state {
        Some(ty) => (
            quote! {
                static __MOONLIT_STATE: ::std::sync::OnceLock<#ty> = ::std::sync::OnceLock::new();
                fn __moonlit_state() -> &'static #ty {
                    __MOONLIT_STATE.get_or_init(<#ty as ::core::default::Default>::default)
                }
            },
            quote! { let ctx = ctx.with_state(__moonlit_state()); },
        ),
        None => (quote! {}, quote! {}),
    };

    // optional plugin-config: validated at init, stored, attached to ctx
    let (config_static, config_init, config_attach) = match &decl.config {
        Some(ty) => (
            quote! {
                static __MOONLIT_PLUGIN_CONFIG: ::std::sync::OnceLock<#ty> = ::std::sync::OnceLock::new();
                fn __moonlit_plugin_config() -> ::core::option::Option<&'static #ty> {
                    __MOONLIT_PLUGIN_CONFIG.get()
                }
            },
            quote! {
                let parsed: #ty = ::moonlit_plugin_sdk::config::from_json_value(&plugin_config)
                    .map_err(|e| ::std::format!("invalid plugin config: {}", e))?;
                ::moonlit_plugin_sdk::PluginConfig::validate(&parsed)?;
                let _ = __MOONLIT_PLUGIN_CONFIG.set(parsed);
            },
            quote! {
                let ctx = match __moonlit_plugin_config() {
                    Some(c) => ctx.with_plugin_config(c),
                    None => ctx,
                };
            },
        ),
        None => (quote! {}, quote! {}, quote! {}),
    };

    quote! {
        #[derive(::core::default::Default)]
        struct MoonlitComponent;

        #state_static
        #config_static

        impl ::moonlit_plugin_sdk::bindings::Guest for MoonlitComponent {
            fn describe() -> ::moonlit_plugin_sdk::bindings::PluginMetadata {
                ::moonlit_plugin_sdk::bindings::PluginMetadata {
                    name: #name.to_string(),
                    version: ::core::env!("CARGO_PKG_VERSION").to_string(),
                    description: ::core::option_env!("CARGO_PKG_DESCRIPTION")
                        .unwrap_or("").to_string(),
                }
            }

            fn init(
                plugin_config: ::std::string::String,
            ) -> ::core::result::Result<
                ::moonlit_plugin_sdk::bindings::PluginMetadata,
                ::std::string::String,
            > {
                #config_init
                ::core::result::Result::Ok(<Self as ::moonlit_plugin_sdk::bindings::Guest>::describe())
            }

            fn list_middlewares(
            ) -> ::std::vec::Vec<::moonlit_plugin_sdk::bindings::MiddlewareInfo> {
                ::std::vec![ #(#list_entries),* ]
            }

            fn execute(
                middleware: ::std::string::String,
                ctx: ::moonlit_plugin_sdk::bindings::ReleaseContext,
                config: ::std::string::String,
            ) -> ::moonlit_plugin_sdk::bindings::MiddlewareResult {
                #[cfg(target_arch = "wasm32")]
                let host = ::moonlit_plugin_sdk::RealHost;
                #[cfg(not(target_arch = "wasm32"))]
                let host = __MoonlitUnavailableHost;
                let ctx = ::moonlit_plugin_sdk::Context::new(
                    &host,
                    ctx.working_directory,
                    ctx.step_name,
                );
                #state_attach
                #config_attach
                match middleware.as_str() {
                    #(#exec_arms)*
                    other => ::moonlit_plugin_sdk::MiddlewareResult::failure(
                        ::std::format!("unknown middleware: {}", other)
                    ).into_wit(),
                }
            }
        }

        // Off-wasm the component entry points are never invoked, but the code
        // must still type-check. This host is only constructed on non-wasm and
        // never called (the exported fns run only inside the component).
        #[cfg(not(target_arch = "wasm32"))]
        struct __MoonlitUnavailableHost;
        #[cfg(not(target_arch = "wasm32"))]
        impl ::moonlit_plugin_sdk::Host for __MoonlitUnavailableHost {
            fn log(&self, _l: ::moonlit_plugin_sdk::LogLevel, _m: &str) {}
            fn get_config(&self, _p: &str) -> ::core::option::Option<::std::string::String> { None }
            fn report_progress(&self, _m: &str) {}
            fn process_run(&self, _cmd: &::moonlit_plugin_sdk::process::ProcessCommand) -> ::core::result::Result<::moonlit_plugin_sdk::process::ProcessOutput, ::std::string::String> { Err("process unavailable".to_string()) }
            fn process_spawn(&self, _cmd: &::moonlit_plugin_sdk::process::ProcessCommand) -> ::core::result::Result<::std::boxed::Box<dyn ::moonlit_plugin_sdk::process::ChildHandle>, ::std::string::String> { Err("process unavailable".to_string()) }
            fn http_send(&self, _req: &::moonlit_plugin_sdk::http::HttpRequestData) -> ::core::result::Result<::moonlit_plugin_sdk::http::HttpResponseData, ::std::string::String> { Err("http unavailable".to_string()) }
            fn env_var(&self, _n: &str) -> ::core::option::Option<::std::string::String> { None }
            fn env_vars(&self) -> ::std::vec::Vec<(::std::string::String, ::std::string::String)> { ::std::vec::Vec::new() }
            fn random_bytes(&self, n: usize) -> ::std::vec::Vec<u8> { ::std::vec![0u8; n] }
            fn monotonic_nanos(&self) -> u64 { 0 }
        }

        ::moonlit_plugin_sdk::export!(MoonlitComponent with_types_in ::moonlit_plugin_sdk::bindings);
    }
}
