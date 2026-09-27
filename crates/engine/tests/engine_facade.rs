use moonlit_engine::engine::Engine;
use moonlit_engine::engine::config::EngineSettings;
use moonlit_engine::engine::error::EngineError;
use moonlit_engine::pipeline::PipelineError;

#[test]
fn engine_new_builds_with_defaults() {
  let eng = Engine::new(EngineSettings::default());
  assert!(eng.is_ok(), "engine construction must succeed with defaults");
}

#[test]
fn exit_codes_match_the_contract() {
  let internal = EngineError::Internal(anyhow::anyhow!("boom"));
  assert_eq!(internal.exit_code(), 1);
  let load = PipelineError::PluginLoad {
    plugin: "p".into(),
    message: "x".into(),
  };
  assert_eq!(load.exit_code(), 3);
  let exec = PipelineError::Execution("x".into());
  assert_eq!(exec.exit_code(), 4);
}

#[test]
fn engine_error_delegates_pipeline_exit_codes() {
  let exec = EngineError::from(PipelineError::Execution("x".into()));
  assert_eq!(exec.exit_code(), 4);
}
