//! `moonlit_plugin!` — generates the WIT `Guest` glue and `export!` for a
//! plugin from a declaration of its name, middlewares, and optional config/state.
//!
//! Emits plugin ABI `moonlit:plugin@0.3.0` metadata: an optional embedded icon
//! (`icon = "…"`) and, per middleware, a JSON Schema for both its typed `Input`
//! and its `Output`.

use base64::Engine as _;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token, Type, braced, bracketed};

/// `moonlit_plugin! { name: "git", icon: "icon.png", config: C, middlewares: [A, B], state: S }`
struct PluginDecl {
  name: LitStr,
  /// Path to a PNG/WebP icon, relative to the author crate's `CARGO_MANIFEST_DIR`.
  icon: Option<LitStr>,
  config: Option<Type>,
  middlewares: Vec<Type>,
  state: Option<Type>,
}

impl Parse for PluginDecl {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let mut name: Option<LitStr> = None;
    let mut icon: Option<LitStr> = None;
    let mut config: Option<Type> = None;
    let mut middlewares: Option<Vec<Type>> = None;
    let mut state: Option<Type> = None;

    while !input.is_empty() {
      let key: Ident = input.parse()?;
      input.parse::<Token![:]>()?;
      match key.to_string().as_str() {
        "name" => name = Some(input.parse()?),
        "icon" => icon = Some(input.parse()?),
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
            format!("unknown moonlit_plugin! field `{other}` (expected name, icon, config, middlewares, state)"),
          ));
        }
      }
      // optional trailing comma between fields
      let _ = input.parse::<Token![,]>();
    }

    Ok(PluginDecl {
      name: name.ok_or_else(|| input.error("missing `name:`"))?,
      icon,
      config,
      middlewares: middlewares.ok_or_else(|| input.error("missing `middlewares:`"))?,
      state,
    })
  }
}

/// Read the icon at expansion time and build a `data:` URI expression for
/// `plugin-metadata.icon`. MIME comes from the extension; unknown extensions are
/// a compile error. The emitted `include_bytes!` is inert (discarded) but makes
/// `rustc` track the file so a changed icon triggers a rebuild.
#[must_use]
fn icon_expr(icon: Option<&LitStr>) -> proc_macro2::TokenStream {
  let Some(lit) = icon else {
    return quote! { ::core::option::Option::None };
  };
  let rel = lit.value();
  let extension = std::path::Path::new(&rel).extension().and_then(std::ffi::OsStr::to_str);
  let mime = if extension == Some("png") {
    "image/png"
  } else if extension == Some("webp") {
    "image/webp"
  } else {
    return syn::Error::new(lit.span(), "icon must be a .png or .webp file").to_compile_error();
  };
  let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") else {
    return syn::Error::new(lit.span(), "CARGO_MANIFEST_DIR unavailable; cannot locate icon").to_compile_error();
  };
  let path = std::path::Path::new(&manifest_dir).join(&rel);
  let bytes = match std::fs::read(&path) {
    Ok(b) => b,
    Err(e) => {
      return syn::Error::new(lit.span(), format!("cannot read icon `{}`: {e}", path.display())).to_compile_error();
    }
  };
  let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
  let data_uri = format!("data:{mime};base64,{b64}");
  quote! {
      {
          // Rebuild tracking only — the encoded string above is the real value.
          const _: &[u8] =
              ::core::include_bytes!(::core::concat!(::core::env!("CARGO_MANIFEST_DIR"), "/", #rel));
          ::core::option::Option::Some(::std::string::String::from(#data_uri))
      }
  }
}

#[proc_macro]
pub fn moonlit_plugin(input: TokenStream) -> TokenStream {
  // Allow trailing braces form `moonlit_plugin! { ... }`.
  let decl = syn::parse_macro_input!(input as PluginDeclInput).0;
  expand(&decl).into()
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

#[must_use]
fn expand(decl: &PluginDecl) -> proc_macro2::TokenStream {
  let name = &decl.name;
  let list_entries = list_entries(&decl.middlewares);
  let icon = icon_expr(decl.icon.as_ref());
  let exec_arms = exec_arms(&decl.middlewares);
  let (state_static, state_attach) = state_tokens(decl.state.as_ref());
  let (config_static, config_init, config_attach) = config_tokens(decl.config.as_ref());

  quote! {
      #[derive(::core::default::Default)]
      struct MoonlitComponent;

      #state_static
      #config_static

      impl ::moonlit_pdk::bindings::Guest for MoonlitComponent {
          fn describe() -> ::moonlit_pdk::bindings::PluginMetadata {
              ::moonlit_pdk::bindings::PluginMetadata {
                  name: #name.to_string(),
                  version: ::core::env!("CARGO_PKG_VERSION").to_string(),
                  description: ::core::option_env!("CARGO_PKG_DESCRIPTION")
                      .unwrap_or("").to_string(),
                  icon: #icon,
              }
          }

          fn init(
              plugin_config: ::std::string::String,
          ) -> ::core::result::Result<
              ::moonlit_pdk::bindings::PluginMetadata,
              ::std::string::String,
          > {
              #config_init
              ::core::result::Result::Ok(<Self as ::moonlit_pdk::bindings::Guest>::describe())
          }

          fn list_middlewares(
          ) -> ::std::vec::Vec<::moonlit_pdk::bindings::MiddlewareInfo> {
              ::std::vec![ #(#list_entries),* ]
          }

          fn execute(
              middleware: ::std::string::String,
              ctx: ::moonlit_pdk::bindings::ReleaseContext,
              config: ::std::string::String,
          ) -> ::moonlit_pdk::bindings::MiddlewareResult {
              #[cfg(target_arch = "wasm32")]
              let host = ::moonlit_pdk::RealHost;
              #[cfg(not(target_arch = "wasm32"))]
              let host = __MoonlitUnavailableHost;
              let ctx = ::moonlit_pdk::Context::new(
                  &host,
                  ctx.working_directory,
                  ctx.step_name,
              );
              #state_attach
              #config_attach
              match middleware.as_str() {
                  #(#exec_arms)*
                  other => ::moonlit_pdk::MiddlewareResult::<()>::failure(
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
      impl ::moonlit_pdk::Host for __MoonlitUnavailableHost {
          fn log(&self, _l: ::moonlit_pdk::LogLevel, _m: &str) {}
          fn get_config(&self, _p: &str) -> ::core::option::Option<::std::string::String> { None }
          fn report_progress(&self, _m: &str) {}
          fn process_run(&self, _cmd: &::moonlit_pdk::process::ProcessCommand) -> ::core::result::Result<::moonlit_pdk::process::ProcessOutput, ::std::string::String> { Err("process unavailable".to_string()) }
          fn process_spawn(&self, _cmd: &::moonlit_pdk::process::ProcessCommand) -> ::core::result::Result<::std::boxed::Box<dyn ::moonlit_pdk::process::ChildHandle>, ::std::string::String> { Err("process unavailable".to_string()) }
          fn http_send(&self, _req: &::moonlit_pdk::http::HttpRequestData) -> ::core::result::Result<::moonlit_pdk::http::HttpResponseData, ::std::string::String> { Err("http unavailable".to_string()) }
          fn env_var(&self, _n: &str) -> ::core::option::Option<::std::string::String> { None }
          fn env_vars(&self) -> ::std::vec::Vec<(::std::string::String, ::std::string::String)> { ::std::vec::Vec::new() }
          fn random_bytes(&self, n: usize) -> ::std::vec::Vec<u8> { ::std::vec![0u8; n] }
          fn monotonic_nanos(&self) -> u64 { 0 }
          fn sleep_nanos(&self, _nanos: u64) {}
      }

      ::moonlit_pdk::export!(MoonlitComponent with_types_in ::moonlit_pdk::bindings);
  }
}

#[must_use]
fn list_entries(mws: &[Type]) -> Vec<proc_macro2::TokenStream> {
  // list-middlewares entries; input-schema / output-schema are the JSON Schemas
  // of each middleware's `Input` / `Output` (draft 2020-12), via the SDK helper.
  mws
    .iter()
    .map(|m| {
      quote! {
          ::moonlit_pdk::bindings::MiddlewareInfo {
              name: <#m as ::moonlit_pdk::Middleware>::NAME.to_string(),
              description: <#m as ::moonlit_pdk::Middleware>::DESCRIPTION.to_string(),
              input_schema: ::core::option::Option::Some(
                  ::moonlit_pdk::__schema_json::<<#m as ::moonlit_pdk::Middleware>::Input>()
              ),
              output_schema: ::core::option::Option::Some(
                  ::moonlit_pdk::__schema_json::<<#m as ::moonlit_pdk::Middleware>::Output>()
              ),
          }
      }
    })
    .collect()
}

#[must_use]
fn exec_arms(mws: &[Type]) -> Vec<proc_macro2::TokenStream> {
  // execute dispatch arms
  mws
    .iter()
    .map(|m| {
      quote! {
          <#m as ::moonlit_pdk::Middleware>::NAME => {
              let input: <#m as ::moonlit_pdk::Middleware>::Input =
                  match ::moonlit_pdk::config::from_json_value(&config) {
                      Ok(c) => c,
                      Err(e) => {
                          return ::moonlit_pdk::MiddlewareResult::<
                              <#m as ::moonlit_pdk::Middleware>::Output,
                          >::failure(
                              ::std::format!("invalid input for `{}`: {}", middleware, e)
                          ).into_wit();
                      }
                  };
              let mw = <#m as ::core::default::Default>::default();
              ::moonlit_pdk::Middleware::execute(&mw, &ctx, input).into_wit()
          }
      }
    })
    .collect()
}

#[must_use]
fn state_tokens(state: Option<&Type>) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
  // optional state static + install
  match state {
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
  }
}

#[must_use]
fn config_tokens(config: Option<&Type>) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream) {
  // optional plugin-config: validated at init, stored, attached to ctx
  match config {
    Some(ty) => (
      quote! {
          static __MOONLIT_PLUGIN_CONFIG: ::std::sync::OnceLock<#ty> = ::std::sync::OnceLock::new();
          fn __moonlit_plugin_config() -> ::core::option::Option<&'static #ty> {
              __MOONLIT_PLUGIN_CONFIG.get()
          }
      },
      quote! {
          let parsed: #ty = ::moonlit_pdk::config::from_json_value(&plugin_config)
              .map_err(|e| ::std::format!("invalid plugin config: {}", e))?;
          ::moonlit_pdk::PluginConfig::validate(&parsed)?;
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
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proc_macro2::Span;

  fn parse_error(tokens: proc_macro2::TokenStream) -> String {
    syn::parse2::<PluginDecl>(tokens)
      .err()
      .expect("declaration must be rejected")
      .to_string()
  }

  fn icon(path: &str) -> String {
    icon_expr(Some(&LitStr::new(path, Span::call_site()))).to_string()
  }

  #[test]
  fn unknown_field_is_rejected_with_the_expected_list() {
    let message = parse_error(quote! { name: "p", bogus: "x" });
    assert!(message.contains("unknown moonlit_plugin! field `bogus`"), "got: {message}");
  }

  #[test]
  fn name_and_middlewares_are_required() {
    assert!(parse_error(quote! { middlewares: [A] }).contains("missing `name:`"));
    assert!(parse_error(quote! { name: "p" }).contains("missing `middlewares:`"));
  }

  #[test]
  fn braced_and_bare_declarations_parse_the_same_fields() {
    let braced = syn::parse2::<PluginDeclInput>(quote! { { name: "p", config: C, middlewares: [A, B], state: S } })
      .expect("braced form parses")
      .0;
    let bare = syn::parse2::<PluginDeclInput>(quote! { name: "p" middlewares: [A] })
      .expect("bare form parses")
      .0;
    assert_eq!(braced.name.value(), "p");
    assert_eq!(braced.middlewares.len(), 2);
    assert!(braced.config.is_some() && braced.state.is_some());
    assert_eq!(bare.middlewares.len(), 1);
    assert!(bare.icon.is_none());
  }

  #[test]
  fn missing_icon_expands_to_none() {
    assert!(icon_expr(None).to_string().contains("None"));
  }

  #[test]
  fn webp_icon_becomes_a_webp_data_uri() {
    let path = std::env::temp_dir().join(format!("moonlit-pdk-macros-{}.webp", std::process::id()));
    std::fs::write(&path, b"RIFF").unwrap();
    let expanded = icon(&path.display().to_string());
    std::fs::remove_file(&path).unwrap();
    assert!(expanded.contains("data:image/webp;base64,UklGRg=="), "got: {expanded}");
  }

  #[test]
  fn unsupported_icon_extension_is_a_compile_error() {
    assert!(icon("icon.gif").contains("icon must be a .png or .webp file"));
    assert!(icon("icon").contains("icon must be a .png or .webp file"));
  }

  #[test]
  fn unreadable_icon_is_a_compile_error() {
    let missing = std::env::temp_dir().join("moonlit-pdk-macros-missing-icon.png");
    assert!(icon(&missing.display().to_string()).contains("cannot read icon"));
  }
}
