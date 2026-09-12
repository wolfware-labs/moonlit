//! Pipeline orchestration: spawn a consumer task that drives the renderer for the whole run,
//! call `load_pipeline` then `run` on the main task, and map the outcome to an exit code.

use moonlit_engine::{Engine, EngineError, PipelineOptions, PipelineSummary};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use moonlit_engine::EngineSettings;

use crate::cli::{OutputMode, RunArgs};
use crate::render::{Header, Renderer};
use crate::{input, render, signal};

/// Run (or, when `load_only`, just load/validate) a pipeline, rendering the event stream.
/// Returns `Ok(Some(summary))` on a completed run, `Ok(None)` for `load_only`, or the engine error.
pub async fn execute(
    engine: &Engine,
    yaml: &str,
    opts: PipelineOptions,
    header: Header,
    renderer: Box<dyn Renderer>,
    cancel: CancellationToken,
    load_only: bool,
) -> Result<Option<PipelineSummary>, EngineError> {
    let (tx, mut rx) = mpsc::channel(256);

    // The consumer owns the renderer + receiver for the whole run (both Send + 'static).
    let consumer = tokio::spawn(async move {
        let mut renderer = renderer;
        renderer.header(&header);
        while let Some(event) = rx.recv().await {
            renderer.handle(&event);
        }
        renderer.finish();
    });

    let load = engine.load_pipeline(yaml, opts, &tx).await;
    let pipeline = match load {
        Ok(p) => p,
        Err(e) => {
            drop(tx); // close the channel so the consumer drains and exits
            let _ = consumer.await;
            return Err(e);
        }
    };

    if load_only {
        // Drop the pipeline before awaiting the consumer: each plugin instance holds a
        // ChannelSink with a *clone* of the event sender (for host.log callbacks), so the
        // channel only closes once the pipeline is gone. Dropping `tx` alone is not enough —
        // without this, `consumer.await` would block forever. (The run path below is fine
        // because `engine.run` consumes and drops the pipeline itself.)
        drop(pipeline);
        drop(tx);
        let _ = consumer.await;
        return Ok(None);
    }

    let result = engine.run(pipeline, tx, cancel).await; // moves tx; closes channel on return
    let _ = consumer.await;
    result.map(Some)
}

/// Map an outcome to a process exit code: success 0, otherwise the engine's category code.
pub fn exit_code(outcome: &Result<Option<PipelineSummary>, EngineError>) -> i32 {
    match outcome {
        Ok(_) => 0,
        Err(e) => e.exit_code(),
    }
}

/// Build a `Header` from resolved inputs and the active stage filter.
fn build_header(resolved: &input::ResolvedInput, stages_filter: &[String]) -> Header {
    let peeked = input::peek_stages(&resolved.yaml);
    let stages = if stages_filter.is_empty() {
        peeked
    } else {
        stages_filter.to_vec()
    };
    Header {
        version: env!("CARGO_PKG_VERSION"),
        name: input::peek_name(&resolved.yaml),
        working_dir: resolved.working_directory.display().to_string(),
        config_file: resolved.chosen_name.clone(),
        stages,
    }
}

/// Render an owned error: a miette report (pretty/plain, spans intact) or a json error object.
/// Generic so it serves both `input::InputError` and `EngineError` — both are
/// `miette::Diagnostic + Send + Sync + 'static`. `code` is precomputed by the caller because
/// the error is consumed here (miette::Report::new takes ownership).
fn report<E>(err: E, code: i32, json: bool)
where
    E: miette::Diagnostic + Send + Sync + 'static,
{
    if json {
        let obj =
            serde_json::json!({ "type": "error", "message": err.to_string(), "exit_code": code });
        println!("{obj}");
    } else {
        eprintln!("{:?}", miette::Report::new(err));
    }
}

/// `moonlit run` (and `--dry-run`, which loads only).
pub async fn run(output: Option<OutputMode>, verbose: bool, args: RunArgs, dry_run: bool) -> i32 {
    let stderr_tty = render::stderr_is_tty();
    let json = render::resolve_mode(output, stderr_tty) == OutputMode::Json;

    let resolved = match input::resolve(args.file, args.working_dir) {
        Ok(r) => r,
        Err(e) => {
            let code = e.exit_code();
            report(e, code, json);
            return code;
        }
    };
    let header = build_header(&resolved, &args.stages);
    let opts = PipelineOptions {
        working_directory: resolved.working_directory.clone(),
        config_file_name: resolved.chosen_name.clone(),
        stages_filter: args.stages.clone(),
        cli_args: args.args.clone(),
        step_timeout: args.step_timeout,
        offline: args.offline,
    };
    let engine = match Engine::new(EngineSettings::default()) {
        Ok(e) => e,
        Err(e) => {
            let code = e.exit_code();
            report(e, code, json);
            return code;
        }
    };
    let cancel = CancellationToken::new();
    signal::spawn_watcher(cancel.clone());
    let renderer = render::for_mode(output, stderr_tty, verbose);

    let outcome = execute(
        &engine,
        &resolved.yaml,
        opts,
        header,
        renderer,
        cancel,
        dry_run,
    )
    .await;
    let code = exit_code(&outcome); // free fn from Step 1, exercised in production here
    if let Err(e) = outcome {
        report(e, code, json);
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::json::JsonRenderer;
    use moonlit_engine::EngineSettings;
    use std::path::Path;

    fn fixture_wasm_url() -> String {
        // cli/ is a sibling of engine/; the prebuilt fixture lives under engine/tests.
        let p =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/fixtures/test_plugin.wasm");
        let p = p.canonicalize().expect("fixture wasm exists");
        format!("file://{}", p.display())
    }

    fn header() -> Header {
        Header {
            version: "0.1.0",
            name: None,
            working_dir: "/tmp".into(),
            config_file: "release.yml".into(),
            stages: vec![],
        }
    }

    fn opts() -> PipelineOptions {
        PipelineOptions {
            config_file_name: "release.yml".to_string(),
            working_directory: std::env::temp_dir(),
            stages_filter: vec![],
            cli_args: vec![],
            step_timeout: None,
            offline: false,
        }
    }

    async fn run_yaml(yaml: &str, load_only: bool) -> Result<Option<PipelineSummary>, EngineError> {
        let engine = Engine::new(EngineSettings::default()).unwrap();
        let renderer = Box::new(JsonRenderer::new(std::io::sink()));
        execute(
            &engine,
            yaml,
            opts(),
            header(),
            renderer,
            CancellationToken::new(),
            load_only,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn successful_run_returns_summary_and_exit_zero() {
        let yaml = format!(
            "name: d\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n",
            fixture_wasm_url()
        );
        let outcome = run_yaml(&yaml, false).await;
        assert_eq!(exit_code(&outcome), 0);
        let summary = outcome.unwrap().unwrap();
        assert!(summary.successful);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failing_step_maps_to_exit_four() {
        let yaml = format!(
            "name: d\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: tp.fail\n",
            fixture_wasm_url()
        );
        let outcome = run_yaml(&yaml, false).await;
        assert_eq!(exit_code(&outcome), 4);
        match outcome {
            Ok(_) => panic!("expected execution error"),
            Err(e) => assert_eq!(e.exit_code(), 4),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unknown_middleware_maps_to_exit_two() {
        let yaml = format!(
            "name: d\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: tp.does-not-exist\n",
            fixture_wasm_url()
        );
        let outcome = run_yaml(&yaml, false).await;
        assert_eq!(exit_code(&outcome), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn load_only_returns_none_and_exit_zero() {
        let yaml = format!(
            "name: d\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n",
            fixture_wasm_url()
        );
        let outcome = run_yaml(&yaml, true).await;
        assert_eq!(exit_code(&outcome), 0);
        assert!(outcome.unwrap().is_none());
    }
}
