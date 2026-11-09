//! `moonlit_plugin!` — generates the WIT `Guest` glue and `export!` for a
//! plugin from a declaration of its name, middlewares, and optional config/state.
//!
//! Emits plugin ABI `moonlit:plugin@0.4.0` metadata: an optional embedded icon
//! (`icon = "…"`) and, per middleware, a JSON Schema for both its typed `Input`
//! and its `Output`.

use base64::Engine as _;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token, Type, braced, bracketed, parenthesized};

/// `moonlit_plugin! { name: "git", icon: "icon.png", config: C, middlewares: [A, B], state: S, permissions: [...] }`
struct PluginDecl {
  name: LitStr,
  /// Path to a PNG/WebP icon, relative to the author crate's `CARGO_MANIFEST_DIR`.
  icon: Option<LitStr>,
  config: Option<Type>,
  middlewares: Vec<Type>,
  state: Option<Type>,
  permissions: Vec<PermissionDecl>,
}

/// One entry of `permissions: [...]`, e.g. `optional exec(["git"]) => "why"`.
struct PermissionDecl {
  optional: bool,
  kind: &'static str,
  values: Vec<LitStr>,
  reason: LitStr,
}

impl Parse for PermissionDecl {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let mut kind_ident: Ident = input.parse()?;
    let optional = kind_ident == "optional";
    if optional {
      kind_ident = input.parse()?;
    }
    let name = kind_ident.to_string();
    if !matches!(name.as_str(), "network" | "exec" | "env" | "filesystem") {
      return Err(syn::Error::new(
        kind_ident.span(),
        format!("unknown permission kind `{name}` (expected network, exec, env, filesystem)"),
      ));
    }
    let args;
    parenthesized!(args in input);
    let (kind, values) = if name == "filesystem" {
      let access: Ident = args
        .parse()
        .map_err(|e: syn::Error| syn::Error::new(e.span(), "filesystem takes read_only or read_write"))?;
      let kind = match access.to_string().as_str() {
        "read_only" => "FilesystemRead",
        "read_write" => "FilesystemWrite",
        _ => return Err(syn::Error::new(access.span(), "filesystem takes read_only or read_write")),
      };
      (kind, Vec::new())
    } else {
      let kind = match name.as_str() {
        "network" => "Network",
        "exec" => "Exec",
        _ => "Env",
      };
      let list;
      let bracket = bracketed!(list in args);
      let values: Vec<LitStr> = list
        .parse_terminated(<LitStr as Parse>::parse, Token![,])?
        .into_iter()
        .collect();
      if values.is_empty() {
        return Err(syn::Error::new(bracket.span.join(), "list at least one value"));
      }
      if let Some(value) = values.iter().find(|v| v.value().contains(['[', ']', '{', '}'])) {
        return Err(syn::Error::new(
          value.span(),
          format!(
            "permission value `{}` uses `[`, `]`, `{{` or `}}`; only `*` and `?` wildcards are allowed",
            value.value()
          ),
        ));
      }
      (kind, values)
    };
    if !input.peek(Token![=>]) {
      return Err(input.error("every permission needs a reason: add => \"why\""));
    }
    input.parse::<Token![=>]>()?;
    let reason: LitStr = input.parse()?;
    if reason.value().trim().is_empty() {
      return Err(syn::Error::new(reason.span(), "a permission's reason cannot be empty"));
    }
    Ok(PermissionDecl {
      optional,
      kind,
      values,
      reason,
    })
  }
}

impl Parse for PluginDecl {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let mut name: Option<LitStr> = None;
    let mut icon: Option<LitStr> = None;
    let mut config: Option<Type> = None;
    let mut middlewares: Option<Vec<Type>> = None;
    let mut state: Option<Type> = None;
    let mut permissions: Vec<PermissionDecl> = Vec::new();

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
        "permissions" => {
          let content;
          bracketed!(content in input);
          let entries = content.parse_terminated(PermissionDecl::parse, Token![,])?;
          permissions = entries.into_iter().collect();
        }
        other => {
          return Err(syn::Error::new(
            key.span(),
            format!("unknown moonlit_plugin! field `{other}` (expected name, icon, config, middlewares, state, permissions)"),
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
      permissions,
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
  let permissions = permission_entries(&decl.permissions);

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
                  permissions: ::std::vec![ #(#permissions),* ],
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
fn permission_entries(permissions: &[PermissionDecl]) -> Vec<proc_macro2::TokenStream> {
  permissions
    .iter()
    .map(|p| {
      let kind = Ident::new(p.kind, proc_macro2::Span::call_site());
      let values = &p.values;
      let reason = &p.reason;
      let optional = p.optional;
      quote! {
          ::moonlit_pdk::bindings::moonlit::plugin::types::PermissionRequest {
              kind: ::moonlit_pdk::bindings::moonlit::plugin::types::PermissionKind::#kind,
              values: ::std::vec![ #(::std::string::String::from(#values)),* ],
              reason: ::std::string::String::from(#reason),
              optional: #optional,
          }
      }
    })
    .collect()
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

  fn permission_error(entry: &proc_macro2::TokenStream) -> String {
    parse_error(quote! { name: "p", middlewares: [A], permissions: [ #entry ] })
  }

  #[test]
  fn a_permission_without_a_reason_is_rejected() {
    let message = permission_error(&quote! { network(["a.com"]) });
    assert!(
      message.contains("every permission needs a reason: add => \"why\""),
      "got: {message}"
    );
  }

  #[test]
  fn an_empty_reason_is_rejected() {
    let message = permission_error(&quote! { network(["a.com"]) => "" });
    assert!(message.contains("a permission's reason cannot be empty"), "got: {message}");
  }

  #[test]
  fn an_unknown_permission_kind_is_rejected() {
    let message = permission_error(&quote! { socket(["a"]) => "why" });
    assert!(
      message.contains("unknown permission kind `socket` (expected network, exec, env, filesystem)"),
      "got: {message}"
    );
  }

  #[test]
  fn an_empty_value_list_is_rejected_for_network_exec_and_env() {
    for kind in [quote! { network }, quote! { exec }, quote! { env }] {
      let message = permission_error(&quote! { #kind([]) => "why" });
      assert!(message.contains("list at least one value"), "got: {message}");
    }
  }

  #[test]
  fn values_with_glob_classes_or_alternatives_are_rejected() {
    for value in ["{git,gh}", "git[0-9]", "a]", "b}"] {
      let message = permission_error(&quote! { exec(["ok", #value]) => "why" });
      assert!(
        message.contains(&format!("`{value}`")) && message.contains("only `*` and `?` wildcards"),
        "got: {message}"
      );
    }
  }

  #[test]
  fn a_filesystem_argument_must_be_read_only_or_read_write() {
    let message = permission_error(&quote! { filesystem(everything) => "why" });
    assert!(message.contains("filesystem takes read_only or read_write"), "got: {message}");
  }

  #[test]
  fn a_filesystem_without_an_identifier_argument_is_rejected() {
    for entry in [quote! { filesystem() => "why" }, quote! { filesystem(["x"]) => "why" }] {
      let message = permission_error(&entry);
      assert!(message.contains("filesystem takes read_only or read_write"), "got: {message}");
    }
  }

  #[test]
  fn an_unknown_kind_without_parentheses_names_the_kind() {
    let message = permission_error(&quote! { socket => "why" });
    assert!(
      message.contains("unknown permission kind `socket` (expected network, exec, env, filesystem)"),
      "got: {message}"
    );
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

  fn permission(entry: proc_macro2::TokenStream) -> PermissionDecl {
    syn::parse2::<PermissionDecl>(entry).expect("permission parses")
  }

  #[test]
  fn each_permission_kind_maps_to_its_wit_variant() {
    let cases = [
      (quote! { filesystem(read_only) => "why" }, "FilesystemRead"),
      (quote! { filesystem(read_write) => "why" }, "FilesystemWrite"),
      (quote! { network(["a.com"]) => "why" }, "Network"),
      (quote! { exec(["git"]) => "why" }, "Exec"),
      (quote! { env(["HOME"]) => "why" }, "Env"),
    ];
    for (entry, kind) in cases {
      assert_eq!(permission(entry).kind, kind);
    }
  }

  #[test]
  fn permission_entries_build_one_request_per_declaration() {
    let decls = [
      permission(quote! { exec(["git"]) => "runs git" }),
      permission(quote! { optional network(["a.com"]) => "fetches" }),
    ];
    let entries = permission_entries(&decls);
    assert_eq!(entries.len(), 2);
    let exec = entries[0].to_string();
    assert!(exec.contains("PermissionKind :: Exec"), "got: {exec}");
    assert!(
      exec.contains("\"git\"") && exec.contains("\"runs git\"") && exec.contains("optional : false"),
      "got: {exec}"
    );
    let network = entries[1].to_string();
    assert!(
      network.contains("PermissionKind :: Network") && network.contains("optional : true"),
      "got: {network}"
    );
  }

  #[test]
  fn expand_exports_the_component_with_its_permissions() {
    let decl =
      syn::parse2::<PluginDeclInput>(quote! { name: "p", middlewares: [A], permissions: [exec(["git"]) => "runs git"] })
        .expect("declaration parses")
        .0;
    let expanded = expand(&decl).to_string();
    assert!(expanded.contains("struct MoonlitComponent"), "got: {expanded}");
    assert!(expanded.contains("PermissionKind :: Exec"), "got: {expanded}");
  }
}
