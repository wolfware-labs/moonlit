//! The middleware authoring trait. A middleware is a `Default` unit struct with
//! a name, a description, a typed config, and an `execute`.

use crate::{Context, MiddlewareResult};

pub trait Middleware: Default {
    /// The `run:` reference name (e.g. `latest-tag`), unique within the plugin.
    const NAME: &'static str;
    /// Shown by `plugin inspect` / `list-middlewares`.
    const DESCRIPTION: &'static str = "";
    /// The step input (config) type; `Default` so an absent block still binds.
    /// `JsonSchema` lets the macro emit `middleware-info.input-schema`.
    /// Use `NoInput` when the middleware reads no configuration.
    type Input: serde::de::DeserializeOwned + Default + schemars::JsonSchema;
    /// The step output type, published for downstream steps. `JsonSchema` lets
    /// the macro emit `middleware-info.output-schema`. Use `NoOutput` when the
    /// middleware publishes nothing.
    type Output: serde::Serialize + schemars::JsonSchema;
    fn execute(&self, ctx: &Context, input: Self::Input) -> MiddlewareResult<Self::Output>;
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::testing::{run, MockHost};

    #[derive(serde::Deserialize, Default, schemars::JsonSchema)]
    #[serde(default)]
    struct GreetInput {
        name: String,
        times: i64,
    }

    #[derive(serde::Serialize, schemars::JsonSchema)]
    struct GreetOutput {
        count: i64,
    }

    #[derive(Default)]
    struct Greet;
    impl Middleware for Greet {
        const NAME: &'static str = "greet";
        const DESCRIPTION: &'static str = "greets";
        type Input = GreetInput;
        type Output = GreetOutput;
        fn execute(&self, ctx: &Context, input: Self::Input) -> MiddlewareResult<Self::Output> {
            ctx.log_info(&format!("hi {}", input.name));
            MiddlewareResult::ok(GreetOutput { count: input.times })
        }
    }

    #[test]
    fn input_derives_a_json_schema() {
        // Proves schemars is wired and a middleware `Input` is derivable; the
        // macro (Task 3) reuses exactly this to emit `middleware-info.input-schema`.
        let schema = serde_json::to_value(schemars::schema_for!(GreetInput)).unwrap();
        assert!(
            schema.pointer("/properties/name").is_some(),
            "schema must expose properties.name; got {schema}"
        );
    }

    #[test]
    fn run_drives_a_middleware_natively() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        // times arrives as a coerced string, proving the caller-side coercion path.
        let input: GreetInput =
            crate::config::from_json_value(r#"{"name":"ada","times":"3"}"#).unwrap();
        let w = run(&Greet, &ctx, input).into_wit();
        assert!(w.successful);
        assert_eq!(host.logs()[0].1, "hi ada");
        assert_eq!(w.output.iter().find(|(k, _)| k == "count").unwrap().1, "3");
    }
}
