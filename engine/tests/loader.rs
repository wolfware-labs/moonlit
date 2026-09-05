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
        config_file_name: "release.yml".to_string(),
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
            assert_eq!(d.message(), "The plugin does not export a middleware named 'nosuch'.")
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
        EngineError::Config(d) => assert_eq!(d.message(), "No plugin is declared with the alias 'other'."),
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

#[tokio::test(flavor = "multi_thread")]
async fn init_domain_error_is_exit_3_plugin_load() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(64);
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {}\n    config:\n      failInit: 'true'\nstages:\n  b:\n    - name: s\n      run: tp.log-and-output\n",
        fixture_url()
    );
    let err = match eng.load_pipeline(&yaml, opts(), &tx).await {
        Ok(_) => panic!("must fail"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 3);
    assert!(matches!(err, EngineError::PluginLoad { .. }));
}

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

fn two_plugin_yaml(url_a: &str, url_b: &str) -> String {
    format!(
        "name: demo\nplugins:\n  - name: a\n    url: {url_a}\n  - name: b\n    url: {url_b}\nstages:\n  build:\n    - name: s1\n      run: a.log-and-output\n    - name: s2\n      run: b.log-and-output\n"
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn loads_multiple_plugins_concurrently() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(256);
    let url = fixture_url();
    let pipeline = eng
        .load_pipeline(&two_plugin_yaml(&url, &url), opts(), &tx)
        .await
        .expect("both load");
    // Declaration order preserved despite concurrent completion.
    assert_eq!(pipeline.plugin_names(), vec!["a", "b"]);
    assert_eq!(pipeline.step_count(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_fixture_middlewares_are_registered() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(64);
    // Each of these loads only if build-time middleware validation finds the middleware.
    for mw in ["tp.fail", "tp.dup-output", "tp.sleep"] {
        eng.load_pipeline(&one_plugin_yaml(mw), opts(), &tx)
            .await
            .unwrap_or_else(|_| panic!("{mw} must be a registered middleware"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn first_failure_aborts_without_panicking() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(256);
    let good = fixture_url();
    let err = match eng
        .load_pipeline(&two_plugin_yaml(&good, "file:///no/such.wasm"), opts(), &tx)
        .await
    {
        Ok(_) => panic!("one bad url fails the load"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 3);
    assert!(matches!(err, EngineError::PluginLoad { .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn middleware_validation_covers_stages_excluded_by_the_filter() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(64);
    // The bad `run:` sits in the `other` stage, which the filter excludes — it must STILL fail,
    // because middleware validation runs over all steps before the stage filter (§7.4).
    let mut o = opts();
    o.stages_filter = vec!["build".to_string()];
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: ok\n      run: tp.log-and-output\n  other:\n    - name: bad\n      run: tp.nosuch\n",
        url = fixture_url()
    );
    let err = match eng.load_pipeline(&yaml, o, &tx).await {
        Ok(_) => panic!("must fail on the excluded stage's bad middleware"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 2);
    match err {
        EngineError::Config(d) => {
            assert_eq!(d.message(), "The plugin does not export a middleware named 'nosuch'.")
        }
        other => panic!("expected Config, got {other:?}"),
    }
}
