//! The README quickstart, compiled for real.
//!
//! `sdk/README.md` is the crates.io front page: it is the first code a new plugin author copies,
//! and nothing else in the workspace compiles it. When the 0.3.0 ABI renamed `Config` -> `Input`
//! and added a typed `Output`, the README kept teaching the removed 0.2 surface.
//!
//! So the example lives here as real code between the two marker comments, and
//! [`readme_quickstart_is_the_compiled_example`] asserts the README's `rust` block is byte-identical
//! to it. Changing one without the other fails the build.

// README-EXAMPLE-START
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
// README-EXAMPLE-END

/// Pull the text between the two marker comments out of this very file.
fn compiled_example() -> String {
    let source = include_str!("readme_quickstart.rs");
    let body = source
        .split_once("// README-EXAMPLE-START\n")
        .expect("start marker present")
        .1;
    body.split_once("// README-EXAMPLE-END")
        .expect("end marker present")
        .0
        .trim()
        .to_string()
}

/// Pull the first ```rust fenced block out of the README.
fn readme_example() -> String {
    let readme = include_str!("../README.md");
    let after_fence = readme
        .split_once("```rust\n")
        .expect("README has a rust code block")
        .1;
    after_fence
        .split_once("```")
        .expect("code block is closed")
        .0
        .trim()
        .to_string()
}

#[test]
fn readme_quickstart_is_the_compiled_example() {
    assert_eq!(
        readme_example(),
        compiled_example(),
        "sdk/README.md's quickstart has drifted from the compiled example in this file. \
         The README is the crates.io front page - update both together."
    );
}
