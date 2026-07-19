//! The committed guest fixture must be a valid wasm component.

const FIXTURE: &[u8] = include_bytes!("fixtures/test_plugin.wasm");

#[test]
fn fixture_is_a_valid_component() {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    // Fails if the bytes are not a valid component (e.g. a core module).
    wasmtime::component::Component::from_binary(&engine, FIXTURE).unwrap();
}
