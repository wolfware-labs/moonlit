use moonlit_engine::engine::Engine;
use moonlit_engine::engine::error::EngineError;

const ASYNC_LIFT_COMPONENT: &str = r#"
(component
  (core module $m
    (func (export "run") (result i32) i32.const 0)
    (func (export "callback") (param i32 i32 i32) (result i32) i32.const 0))
  (core instance $i (instantiate $m))
  (type $run (func async))
  (func (export "run") (type $run)
    (canon lift (core func $i "run") async (callback (core func $i "callback")))))
"#;

#[test]
fn fixture_is_a_valid_async_component() {
  let mut config = wasmtime::Config::new();
  config.wasm_component_model(true);
  config.wasm_component_model_async(true);
  let engine = wasmtime::Engine::new(&config).unwrap();
  wasmtime::component::Component::new(&engine, ASYNC_LIFT_COMPONENT).unwrap();
}

#[test]
fn engine_rejects_async_components() {
  let engine = Engine::try_default().unwrap();
  let bytes = wat::parse_str(ASYNC_LIFT_COMPONENT).unwrap();
  let err = engine.load_component(&bytes).unwrap_err();
  let EngineError::ComponentLoad(message) = err else {
    panic!("expected ComponentLoad, got {err:?}");
  };
  assert!(message.contains("async"), "{message}");
}
