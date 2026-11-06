//! Verifies `permissions:` in `moonlit_plugin!` reaches `plugin-metadata.permissions`.

use moonlit_pdk::bindings::Guest;
use moonlit_pdk::bindings::moonlit::plugin::types::PermissionKind;
use moonlit_pdk::prelude::*;

#[derive(Default)]
struct Noop;
impl Middleware for Noop {
  const NAME: &'static str = "noop";
  const DESCRIPTION: &'static str = "does nothing";
  type Input = NoInput;
  type Output = NoOutput;
  fn execute(&self, _ctx: &Context, _input: Self::Input) -> MiddlewareResult<Self::Output> {
    MiddlewareResult::ok(NoOutput {})
  }
}

moonlit_plugin! {
    name: "perms-sample",
    middlewares: [Noop],
    permissions: [
        network(["api.github.com", "uploads.github.com"]) => "Creates the release and uploads its assets.",
        env(["GITHUB_*"]) => "Reads GITHUB_TOKEN to call the API.",
        filesystem(read_only) => "Reads the changelog and built artifacts.",
        optional exec(["git"]) => "Only for the tag middleware.",
        filesystem(read_write) => "Writes the changelog.",
    ],
}

#[test]
fn describe_carries_declared_permissions_in_order() {
  let meta = <MoonlitComponent as Guest>::describe();
  let kinds: Vec<_> = meta.permissions.iter().map(|p| p.kind).collect();
  assert_eq!(
    kinds,
    [
      PermissionKind::Network,
      PermissionKind::Env,
      PermissionKind::FilesystemRead,
      PermissionKind::Exec,
      PermissionKind::FilesystemWrite
    ]
  );
  assert_eq!(meta.permissions[0].values, ["api.github.com", "uploads.github.com"]);
  assert_eq!(meta.permissions[2].values, Vec::<String>::new());
  assert!(meta.permissions[3].optional && !meta.permissions[0].optional);
  assert_eq!(meta.permissions[1].reason, "Reads GITHUB_TOKEN to call the API.");
}

#[test]
fn init_returns_the_same_permissions() {
  let meta = <MoonlitComponent as Guest>::init("{}".to_string()).unwrap();
  assert_eq!(meta.permissions.len(), 5);
}
