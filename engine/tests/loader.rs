use std::path::Path;

use moonlit_engine::{Engine, EngineError, EngineSettings, PipelineOptions};
use tokio::sync::mpsc::channel;

fn fixture_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_plugin.wasm");
    format!("file://{}", p.display())
}

fn opts() -> PipelineOptions {
    PipelineOptions {
        working_directory: std::env::temp_dir(),
        stages_filter: vec![],
        cli_args: vec![],
        step_timeout: None,
        offline: false,
    }
}

fn one_plugin_yaml(run: &str) -> String {
    format!(
        "name: demo\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: {}\n",
        fixture_url(),
        run
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn load_pipeline_happy_path_emits_resolving_then_ready() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, mut rx) = channel(64);
    let pipeline = eng
        .load_pipeline(&one_plugin_yaml("tp.log-and-output"), opts(), &tx)
        .await
        .expect("load ok");
    assert_eq!(pipeline.step_count(), 1);
    assert_eq!(pipeline.plugin_names(), vec!["tp"]);

    // load_pipeline has fully awaited, so every event is already buffered. Drain non-blocking with
    // try_recv: the returned `pipeline` keeps a Sender alive via each plugin's ChannelSink, so the
    // channel never closes and `recv().await` would hang.
    let mut names_seen = Vec::new();
    let mut ready_version = None;
    while let Ok(ev) = rx.try_recv() {
        use moonlit_engine::PipelineEvent::*;
        match ev {
            PluginResolving { name, .. } => names_seen.push(("resolving", name)),
            PluginReady { name, version, .. } => {
                names_seen.push(("ready", name));
                ready_version = Some(version);
            }
            _ => {}
        }
    }
    // Resolving precedes Ready for tp.
    let ri = names_seen
        .iter()
        .position(|(k, _)| *k == "resolving")
        .unwrap();
    let yi = names_seen.iter().position(|(k, _)| *k == "ready").unwrap();
    assert!(ri < yi, "Resolving must precede Ready: {names_seen:?}");
    assert!(ready_version.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn middleware_not_found_is_exit_2_config_error() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(64);
    let err = match eng
        .load_pipeline(&one_plugin_yaml("tp.nosuch"), opts(), &tx)
        .await
    {
        Ok(_) => panic!("must fail"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 2);
    match err {
        EngineError::Config(d) => {
            assert_eq!(d.message(), "Middleware with name 'nosuch' not found.")
        }
        other => panic!("expected Config, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn plugin_not_found_is_exit_2_config_error() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(64);
    // tp loads fine, but the step references an undeclared alias `other`.
    let err = match eng
        .load_pipeline(&one_plugin_yaml("other.log-and-output"), opts(), &tx)
        .await
    {
        Ok(_) => panic!("must fail"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 2);
    match err {
        EngineError::Config(d) => assert_eq!(d.message(), "Plugin 'other' not found."),
        other => panic!("expected Config, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn resolve_failure_is_exit_3_plugin_load() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(64);
    let yaml = "name: d\nplugins:\n  - name: tp\n    url: file:///no/such/plugin.wasm\nstages:\n  b:\n    - name: s\n      run: tp.x\n";
    let err = match eng.load_pipeline(yaml, opts(), &tx).await {
        Ok(_) => panic!("must fail"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 3);
    assert!(matches!(err, EngineError::PluginLoad { .. }));
}

// NOTE: the init-domain-error → exit-3 test lives in Task 7, not here. The loader serializes
// plugin config as JSON strings (§4.2 scalars are strings), but the current fixture's `init` only
// fails on the JSON boolean `"failInit":true`. Task 7 rebuilds the fixture (for the ABI version
// bump) and enhances `init` to also fail on the string form `"failInit":"true"` — the form the
// loader actually delivers — and adds the `init_domain_error_is_exit_3_plugin_load` test there.

#[tokio::test(flavor = "multi_thread")]
async fn zero_plugins_is_exit_2_config_error() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(64);
    let yaml = "name: d\nstages:\n  b:\n    - name: s\n      run: p.x\n";
    let err = match eng.load_pipeline(yaml, opts(), &tx).await {
        Ok(_) => panic!("must fail"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 2);
    assert!(matches!(err, EngineError::Config(_)));
}
