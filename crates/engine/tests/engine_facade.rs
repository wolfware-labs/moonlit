use moonlit_engine::{Engine, EngineError, EngineSettings};

#[test]
fn engine_new_builds_with_defaults() {
    let eng = Engine::new(EngineSettings::default());
    assert!(
        eng.is_ok(),
        "engine construction must succeed with defaults"
    );
}

#[test]
fn exit_codes_match_the_contract() {
    let internal = EngineError::Internal(anyhow::anyhow!("boom"));
    assert_eq!(internal.exit_code(), 1);
    let load = EngineError::PluginLoad {
        plugin: "p".into(),
        message: "x".into(),
    };
    assert_eq!(load.exit_code(), 3);
    let exec = EngineError::Execution("x".into());
    assert_eq!(exec.exit_code(), 4);
}
