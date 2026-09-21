use crate::cli::{OutputMode, RunArgs};
use crate::render::{Header, Renderer};
use crate::{input, render, signal};
use moonlit_engine::engine::Engine;
use moonlit_engine::engine::config::EngineSettings;
use moonlit_engine::engine::error::EngineError;
use moonlit_engine::pipeline::{PipelineOptions, PipelineSummary};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

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
        drop(pipeline);
        drop(tx);
        let _ = consumer.await;
        return Ok(None);
    }

    let result = engine.run(pipeline, tx, cancel).await; // moves tx; closes channel on return
    let _ = consumer.await;
    result.map(Some)
}

pub fn exit_code(outcome: &Result<Option<PipelineSummary>, EngineError>) -> i32 {
    match outcome {
        Ok(_) => 0,
        Err(e) => e.exit_code(),
    }
}

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

pub async fn run(output: Option<OutputMode>, verbose: bool, args: RunArgs) -> i32 {
    let json = render::resolve_mode(output) == OutputMode::Json;

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
    let renderer = render::for_mode(output, verbose);

    let outcome = execute(
        &engine,
        &resolved.yaml,
        opts,
        header,
        renderer,
        cancel,
        args.dry_run,
    )
    .await;
    let code = exit_code(&outcome);
    if let Err(e) = outcome {
        report(e, code, json);
    }
    code
}
