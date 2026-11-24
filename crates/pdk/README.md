# moonlit-pdk

SDK for writing [Moonlit](https://github.com/wolfware-labs/moonlit) release-pipeline
plugins as WebAssembly (`wasm32-wasip2`) components.

A Moonlit plugin is a `cdylib` that implements one or more middlewares over a `Context`
(HTTP, process exec, filesystem, environment, logging, and clock capabilities, each
gated by the host's deny-by-default sandbox). The `moonlit_plugin!` macro (from
`moonlit-pdk-macros`, re-exported here) wires your middlewares and optional
plugin config/state into the generated component entrypoints.

```rust
use moonlit_pdk::prelude::*;

/// Input for the `greet` middleware, bound from the step's config block.
/// `JsonSchema` is what lets `moonlit plugin inspect` and the registry
/// document these fields.
#[derive(Deserialize, Default, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
struct GreetInput {
    /// Who to greet; defaults to "world" when empty.
    name: String,
}

/// Output published for downstream steps to read as
/// `steps.<step>.outputs.greeting`.
#[derive(Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GreetOutput {
    /// The greeting message, e.g. `hello, world`.
    greeting: String,
}

#[derive(Default)]
struct Greet;

impl Middleware for Greet {
    const NAME: &'static str = "greet";
    const DESCRIPTION: &'static str = "logs and returns a greeting";
    type Input = GreetInput;
    type Output = GreetOutput;

    fn execute(&self, ctx: &Context, input: Self::Input) -> MiddlewareResult<Self::Output> {
        let who = if input.name.is_empty() {
            "world".to_string()
        } else {
            input.name
        };
        ctx.log_info(&format!("greeting {who}"));
        MiddlewareResult::ok(GreetOutput {
            greeting: format!("hello, {who}"),
        })
    }
}

moonlit_plugin! { name: "greet-plugin", middlewares: [Greet] }
```

Each middleware declares two types. `Input` is deserialized from the step's config
block (a `#[serde(default)]` struct is the idiomatic "all fields optional" choice);
`Output` is serialized and published for later steps to read as
`steps.<step>.outputs.<field>`. Use `NoInput` or `NoOutput` when a middleware takes
no configuration or publishes nothing. Both types derive `schemars::JsonSchema`, and
the macro emits those schemas into the component so `moonlit plugin inspect` and the
registry can document the plugin without running it.

Alongside `moonlit-pdk`, a plugin crate needs `serde` (with `derive`),
`serde_json`, and `schemars = "1"`. `moonlit plugin new` scaffolds all of this.
The example above is what it generates.

Build with `cargo build --target wasm32-wasip2 --release` (requires
`rustup target add wasm32-wasip2`).

## Declaring permissions

A plugin has no access to the network, programs, environment, or files unless the pipeline
grants it. Declare what the plugin needs with `permissions:`; `moonlit add` shows these to
the user and writes them to `release.yml`, and `moonlit run` refuses to start if the
pipeline does not grant the required ones.

```rust
moonlit_plugin! {
    name: "github",
    middlewares: [CreateRelease, CommentPr],
    permissions: [
        network(["api.github.com", "uploads.github.com"]) => "Creates the release and uploads its assets.",
        env(["GITHUB_*"]) => "Reads GITHUB_TOKEN to call the API.",
        filesystem(read_only) => "Reads the changelog and built artifacts.",
        optional exec(["git"]) => "Only needed to read the latest tag from a local clone.",
    ],
}
```

- `network([...])`, `exec([...])` and `env([...])` take a non-empty list of strings: hosts,
  program names, and environment variable names or globs.
  Values are literal or use the `*` and `?` wildcards; `[`, `]`, `{`, `}` and `\` do not compile.
  A wildcard never matches `/` (`**` means what `*` does), so `exec(["git*"])` covers `git` and `git-lfs`
  but no program given by path; grant a path by spelling its folders out (`/usr/bin/*`). A program path
  that contains `..` is always refused.
- A program the plugin runs starts in the working directory, or in a folder inside it when the plugin names
  one. It sees a fixed set of host variables (`PATH`, `HOME`, `TMPDIR`, `LANG`, `LC_*`, `TZ`, `USER`,
  `LOGNAME`, or their Windows counterparts), the variables granted under `env`, and the ones the plugin sets
  on the command. The plugin may set only names granted under `env`, so declare the ones it passes on
  (`env(["GIT_AUTHOR_*"])`), and never `PATH`, `LD_*`, `DYLD_*`, `GCONV_PATH` or `LOCPATH` (on Windows
  also `PATHEXT` and `COMSPEC`), nor a name that is empty or holds `=`.
- Refusals are counted per call: past 100 distinct ones the call fails, even when optional permissions cover
  them all.
- `filesystem(read_only)` or `filesystem(read_write)` takes no list.
- Every entry needs `=> "reason"`, a non-empty string shown to the user. A permission
  without one does not compile.
- A leading `optional` marks a permission the plugin can run without. It is shown
  separately and is not granted unless the user allows it. When it is not granted, a call
  that needs it is refused and the plugin gets the error, so it should carry on without it.
  The step then reports a warning instead of failing. Say in the reason when the
  permission is needed.
- Omit `permissions:` for a plugin that needs none.

The CLI checks the same rules when it loads a plugin, so a component built without the macro
that breaks one of them fails to load.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
