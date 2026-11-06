use std::path::Path;

use moonlit_engine::engine::Engine;
use moonlit_engine::engine::config::EngineSettings;
use moonlit_engine::pipeline::manifest::PipelineManifest;
use moonlit_engine::pipeline::{Pipeline, PipelineError, PipelineEvent, PipelineOptions};
use moonlit_engine::plugin::publish::PLUGIN_WORLD;
use tokio::sync::mpsc::channel;

fn fixture_url() -> String {
  let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_plugin.wasm");
  format!("file://{}", p.display())
}

fn manifest(yaml: &str) -> PipelineManifest {
  PipelineManifest {
    working_dir: std::env::temp_dir(),
    file_name: "release.yml".to_string(),
    content: yaml.to_string(),
  }
}

fn opts() -> PipelineOptions {
  PipelineOptions {
    stages_filter: vec![],
    cli_args: vec![],
    step_timeout: None,
    offline: false,
  }
}

fn one_plugin_yaml(run: &str) -> String {
  format!(
    "name: demo\nplugins:\n  - name: tp\n    url: {}\n    permissions:\n      network: [api.example.com]\nstages:\n  build:\n    - name: s1\n      run: {}\n",
    fixture_url(),
    run
  )
}

#[tokio::test(flavor = "multi_thread")]
async fn load_pipeline_happy_path_emits_resolving_then_ready() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, mut rx) = channel(64);
  let pipeline = Pipeline::load(&eng, &manifest(&one_plugin_yaml("tp.log-and-output")), opts(), &tx)
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
    use PipelineEvent::*;
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
  let ri = names_seen.iter().position(|(k, _)| *k == "resolving").unwrap();
  let yi = names_seen.iter().position(|(k, _)| *k == "ready").unwrap();
  assert!(ri < yi, "Resolving must precede Ready: {names_seen:?}");
  assert!(ready_version.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn middleware_not_found_is_exit_2_config_error() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let Err(err) = Pipeline::load(&eng, &manifest(&one_plugin_yaml("tp.nosuch")), opts(), &tx).await else {
    panic!("must fail");
  };
  assert_eq!(err.exit_code(), 2);
  match err {
    PipelineError::Config(d) => {
      assert_eq!(d.message(), "The plugin does not export a middleware named 'nosuch'.");
    }
    other => panic!("expected Config, got {other:?}"),
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn plugin_not_found_is_exit_2_config_error() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  // tp loads fine, but the step references an undeclared alias `other`.
  let Err(err) = Pipeline::load(&eng, &manifest(&one_plugin_yaml("other.log-and-output")), opts(), &tx).await else {
    panic!("must fail");
  };
  assert_eq!(err.exit_code(), 2);
  match err {
    PipelineError::Config(d) => {
      assert_eq!(d.message(), "No plugin is declared with the alias 'other'.");
    }
    other => panic!("expected Config, got {other:?}"),
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn resolve_failure_is_exit_3_plugin_load() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let yaml =
    "name: d\nplugins:\n  - name: tp\n    url: file:///no/such/plugin.wasm\nstages:\n  b:\n    - name: s\n      run: tp.x\n";
  let Err(err) = Pipeline::load(&eng, &manifest(yaml), opts(), &tx).await else {
    panic!("must fail");
  };
  assert_eq!(err.exit_code(), 3);
  assert!(matches!(err, PipelineError::PluginLoad { .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn init_domain_error_is_exit_3_plugin_load() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let yaml = format!(
    "name: d\nplugins:\n  - name: tp\n    url: {}\n    permissions:\n      network: [api.example.com]\n    config:\n      failInit: 'true'\nstages:\n  b:\n    - name: s\n      run: tp.log-and-output\n",
    fixture_url()
  );
  let Err(err) = Pipeline::load(&eng, &manifest(&yaml), opts(), &tx).await else {
    panic!("must fail");
  };
  assert_eq!(err.exit_code(), 3);
  assert!(matches!(err, PipelineError::PluginLoad { .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn zero_plugins_is_exit_2_config_error() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let yaml = "name: d\nstages:\n  b:\n    - name: s\n      run: p.x\n";
  let Err(err) = Pipeline::load(&eng, &manifest(yaml), opts(), &tx).await else {
    panic!("must fail");
  };
  assert_eq!(err.exit_code(), 2);
  assert!(matches!(err, PipelineError::Config(_)));
}

fn two_plugin_yaml(url_a: &str, url_b: &str) -> String {
  format!(
    "name: demo\nplugins:\n  - name: a\n    url: {url_a}\n    permissions:\n      network: [api.example.com]\n  - name: b\n    url: {url_b}\n    permissions:\n      network: [api.example.com]\nstages:\n  build:\n    - name: s1\n      run: a.log-and-output\n    - name: s2\n      run: b.log-and-output\n"
  )
}

#[tokio::test(flavor = "multi_thread")]
async fn loads_multiple_plugins_concurrently() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(256);
  let url = fixture_url();
  let pipeline = Pipeline::load(&eng, &manifest(&two_plugin_yaml(&url, &url)), opts(), &tx)
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
    let pipeline = Pipeline::load(&eng, &manifest(&one_plugin_yaml(mw)), opts(), &tx)
      .await
      .unwrap_or_else(|_| panic!("{mw} must be a registered middleware"));
    assert_eq!(pipeline.step_count(), 1);
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn first_failure_aborts_without_panicking() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(256);
  let good = fixture_url();
  let Err(err) = Pipeline::load(&eng, &manifest(&two_plugin_yaml(&good, "file:///no/such.wasm")), opts(), &tx).await else {
    panic!("one bad url fails the load");
  };
  assert_eq!(err.exit_code(), 3);
  assert!(matches!(err, PipelineError::PluginLoad { .. }));
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
    "name: d\nplugins:\n  - name: tp\n    url: {url}\n    permissions:\n      network: [api.example.com]\nstages:\n  build:\n    - name: ok\n      run: tp.log-and-output\n  other:\n    - name: bad\n      run: tp.nosuch\n",
    url = fixture_url()
  );
  let Err(err) = Pipeline::load(&eng, &manifest(&yaml), o, &tx).await else {
    panic!("must fail on the excluded stage's bad middleware");
  };
  assert_eq!(err.exit_code(), 2);
  match err {
    PipelineError::Config(d) => {
      assert_eq!(d.message(), "The plugin does not export a middleware named 'nosuch'.");
    }
    other => panic!("expected Config, got {other:?}"),
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_every_incompatible_plugin() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(256);
  let dir = tempfile::tempdir().unwrap();
  let old = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_plugin_v0_3.wasm");
  let first = dir.path().join("old_one.wasm");
  let second = dir.path().join("old_two.wasm");
  std::fs::copy(&old, &first).unwrap();
  std::fs::copy(&old, &second).unwrap();
  let yaml = format!(
    "name: demo\nplugins:\n  - name: a\n    url: file://{}\n  - name: ok\n    url: {}\n    permissions:\n      network: [api.example.com]\n  - name: b\n    url: file://{}\nstages:\n  build:\n    - name: s1\n      run: ok.log-and-output\n",
    first.display(),
    fixture_url(),
    second.display()
  );
  let Err(err) = Pipeline::load(&eng, &manifest(&yaml), opts(), &tx).await else {
    panic!("must fail");
  };
  assert_eq!(err.exit_code(), 3);
  let PipelineError::IncompatiblePlugins(found) = err else {
    panic!("expected IncompatiblePlugins, got {err:?}");
  };
  assert_eq!(found.len(), 2);
  let names: Vec<&str> = found.iter().map(|p| p.name.as_str()).collect();
  assert_eq!(names, vec!["a", "b"]);
  for plugin in &found {
    assert_eq!(plugin.plugin_world.as_deref(), Some("moonlit:plugin@0.3.0"));
    assert_eq!(plugin.engine_world, PLUGIN_WORLD);
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn incompatible_plugins_keep_declaration_order() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(256);
  let dir = tempfile::tempdir().unwrap();
  let old = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_plugin_v0_3.wasm");
  let first = dir.path().join("old_one.wasm");
  let second = dir.path().join("old_two.wasm");
  std::fs::copy(&old, &first).unwrap();
  std::fs::copy(&old, &second).unwrap();
  let yaml = format!(
    "name: demo\nplugins:\n  - name: b\n    url: file://{}\n  - name: a\n    url: file://{}\nstages:\n  build:\n    - name: s1\n      run: a.log-and-output\n",
    second.display(),
    first.display()
  );
  let Err(PipelineError::IncompatiblePlugins(found)) = Pipeline::load(&eng, &manifest(&yaml), opts(), &tx).await else {
    panic!("must fail with IncompatiblePlugins");
  };
  let names: Vec<&str> = found.iter().map(|p| p.name.as_str()).collect();
  assert_eq!(names, vec!["b", "a"]);
}

fn yaml_with_permissions(permissions: &str) -> String {
  format!(
    "name: demo\nplugins:\n  - name: tp\n    url: {}\n{permissions}stages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n",
    fixture_url()
  )
}

#[tokio::test(flavor = "multi_thread")]
async fn refuses_when_required_permission_not_granted() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let Err(err) = Pipeline::load(&eng, &manifest(&yaml_with_permissions("")), opts(), &tx).await else {
    panic!("must fail");
  };
  assert_eq!(err.exit_code(), 2);
  assert_eq!(err.to_string(), "release file does not grant what its plugins need");
  let PipelineError::MissingGrants(found) = err else {
    panic!("expected MissingGrants, got {err:?}");
  };
  assert_eq!(found.len(), 1);
  assert!(found[0].message().contains("api.example.com"), "{}", found[0].message());
  assert!(
    found[0].message().contains("Calls the example API."),
    "{}",
    found[0].message()
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_the_missing_grant_of_every_plugin() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let yaml = format!(
    "name: demo\nplugins:\n  - name: one\n    url: {url}\n  - name: granted\n    url: {url}\n    permissions:\n      network: [api.example.com]\n  - name: two\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: one.log-and-output\n",
    url = fixture_url()
  );
  let Err(PipelineError::MissingGrants(found)) = Pipeline::load(&eng, &manifest(&yaml), opts(), &tx).await else {
    panic!("must fail with MissingGrants");
  };
  assert_eq!(found.len(), 2);
  assert!(
    found[0]
      .message()
      .starts_with("plugin 'one' needs to connect to the internet"),
    "{}",
    found[0].message()
  );
  assert!(
    found[1]
      .message()
      .starts_with("plugin 'two' needs to connect to the internet"),
    "{}",
    found[1].message()
  );
  for diagnostic in &found {
    assert!(diagnostic.message().contains("api.example.com"), "{}", diagnostic.message());
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn optional_permission_not_required() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let yaml = yaml_with_permissions("    permissions:\n      network: [\"api.example.com\"]\n");
  let pipeline = Pipeline::load(&eng, &manifest(&yaml), opts(), &tx).await.expect("loads");
  assert_eq!(pipeline.step_count(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn granted_glob_covers_required() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let yaml = yaml_with_permissions("    permissions:\n      network: [\"*.example.com\"]\n");
  let pipeline = Pipeline::load(&eng, &manifest(&yaml), opts(), &tx).await.expect("loads");
  assert_eq!(pipeline.step_count(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_grant_is_reported_before_init_runs() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let yaml = format!(
    "name: d\nplugins:\n  - name: tp\n    url: {}\n    config:\n      failInit: 'true'\nstages:\n  b:\n    - name: s\n      run: tp.log-and-output\n",
    fixture_url()
  );
  let Err(err) = Pipeline::load(&eng, &manifest(&yaml), opts(), &tx).await else {
    panic!("must fail");
  };
  assert_eq!(err.exit_code(), 2);
  assert!(matches!(err, PipelineError::MissingGrants(_)), "{err:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_grant_of_one_plugin_stops_init_of_every_plugin() {
  let eng = Engine::new(EngineSettings::default()).unwrap();
  let (tx, _rx) = channel(64);
  let yaml = format!(
    "name: d\nplugins:\n  - name: granted\n    url: {url}\n    permissions:\n      network: [api.example.com]\n    config:\n      failInit: 'true'\n  - name: bare\n    url: {url}\nstages:\n  b:\n    - name: s\n      run: granted.log-and-output\n",
    url = fixture_url()
  );
  let Err(err) = Pipeline::load(&eng, &manifest(&yaml), opts(), &tx).await else {
    panic!("must fail");
  };
  let PipelineError::MissingGrants(found) = err else {
    panic!("expected MissingGrants, got {err:?}");
  };
  assert_eq!(found.len(), 1);
  assert!(found[0].message().starts_with("plugin 'bare'"), "{}", found[0].message());
}
