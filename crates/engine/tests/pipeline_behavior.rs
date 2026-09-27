use std::path::Path;
use std::time::Duration;

use moonlit_engine::engine::Engine;
use moonlit_engine::engine::config::EngineSettings;
use moonlit_engine::pipeline::manifest::PipelineManifest;
use moonlit_engine::pipeline::{Pipeline, PipelineError, PipelineEvent, PipelineOptions};
use tokio::sync::mpsc::{Receiver, channel};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE: &[u8] = include_bytes!("fixtures/test_plugin.wasm");

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

fn opts(cli_args: Vec<(String, String)>) -> PipelineOptions {
  PipelineOptions {
    stages_filter: vec![],
    cli_args,
    step_timeout: None,
    offline: false,
  }
}

fn engine(cache_dir: &Path) -> Engine {
  Engine::new(EngineSettings {
    cache_dir: Some(cache_dir.to_path_buf()),
    tag_ttl: Duration::from_mins(15),
  })
  .unwrap()
}

fn drain(rx: &mut Receiver<PipelineEvent>) -> Vec<PipelineEvent> {
  let mut out = Vec::new();
  while let Ok(event) = rx.try_recv() {
    out.push(event);
  }
  out
}

fn conditional_yaml() -> String {
  format!(
    "arguments:\n  version: '1'\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: gated\n      run: tp.log-and-output\n      condition: $(args:version) == 2\n",
    fixture_url()
  )
}

async fn gated_step_skipped(cli_args: Vec<(String, String)>) -> bool {
  let cache = tempfile::tempdir().unwrap();
  let eng = engine(cache.path());
  let (tx, _rx) = channel(256);
  let pipeline = Pipeline::load(&eng, &manifest(&conditional_yaml()), opts(cli_args), &tx)
    .await
    .expect("load ok");
  let summary = pipeline.run(tx, CancellationToken::new()).await.expect("run ok");
  summary.steps[0].skipped
}

#[tokio::test(flavor = "multi_thread")]
async fn declared_arguments_apply_without_cli_overrides() {
  assert!(gated_step_skipped(vec![]).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn cli_arguments_override_declared_arguments() {
  assert!(!gated_step_skipped(vec![("version".to_string(), "2".to_string())]).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn http_plugins_report_download_progress_then_come_from_the_cache() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/tp.wasm"))
    .respond_with(ResponseTemplate::new(200).set_body_bytes(FIXTURE))
    .expect(1)
    .mount(&server)
    .await;
  let yaml = format!(
    "plugins:\n  - name: tp\n    url: {}/tp.wasm\nstages:\n  build:\n    - run: tp.log-and-output\n",
    server.uri()
  );
  let cache = tempfile::tempdir().unwrap();
  let eng = engine(cache.path());

  let (tx, mut rx) = channel(1024);
  let first = Pipeline::load(&eng, &manifest(&yaml), opts(vec![]), &tx)
    .await
    .expect("download ok");
  let events = drain(&mut rx);
  let fixture_len = u64::try_from(FIXTURE.len()).unwrap();
  assert!(events.iter().any(|event| matches!(
    event,
    PipelineEvent::PluginPullProgress { name, received, .. } if name == "tp" && *received == fixture_len
  )));
  assert!(events.iter().any(|event| matches!(
    event,
    PipelineEvent::PluginReady { name, cached: false, .. } if name == "tp"
  )));
  drop(first);

  let (tx, mut rx) = channel(1024);
  let second = Pipeline::load(&eng, &manifest(&yaml), opts(vec![]), &tx)
    .await
    .expect("cache hit ok");
  let events = drain(&mut rx);
  assert!(
    !events
      .iter()
      .any(|event| matches!(event, PipelineEvent::PluginPullProgress { .. }))
  );
  assert!(
    events
      .iter()
      .any(|event| matches!(event, PipelineEvent::PluginReady { cached: true, .. }))
  );
  assert_eq!(second.step_count(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_poisoned_plugin_stops_the_run_without_continue_on_error() {
  let yaml = format!(
    "plugins:\n  - name: a\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: a.boom\n      continueOnError: \"true\"\n    - name: s2\n      run: a.log-and-output\n    - name: s3\n      run: a.log-and-output\n",
    fixture_url()
  );
  let cache = tempfile::tempdir().unwrap();
  let eng = engine(cache.path());
  let (tx, _rx) = channel(256);
  let pipeline = Pipeline::load(&eng, &manifest(&yaml), opts(vec![]), &tx)
    .await
    .expect("load ok");

  let Err(PipelineError::Execution(message)) = pipeline.run(tx, CancellationToken::new()).await else {
    panic!("a poisoned plugin without continueOnError must stop the run");
  };
  assert_eq!(message, "Plugin 'a' unavailable after an earlier failure in this run.");
}
