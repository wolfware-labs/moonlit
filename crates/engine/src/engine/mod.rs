pub mod config;
pub mod error;

use crate::cache::{Cache, SystemClock};
use crate::engine::config::EngineSettings;
use crate::engine::error::EngineError;
use std::sync::Arc;
use std::time::Duration;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Store};
use wasmtime_wasi::WasiView;
use wasmtime_wasi_http::WasiHttpView;

#[derive(Clone)]
pub struct Engine {
  wasm_engine: wasmtime::Engine,
  cache: Arc<Cache>,
  tag_ttl: Duration,
}

impl Engine {
  pub fn new(settings: EngineSettings) -> Result<Self, EngineError> {
    let wasm_engine = Self::build_engine()?;
    let cache = match settings.cache_dir {
      Some(dir) => Cache::with_root_and_clock(dir, Box::new(SystemClock)),
      None => Cache::new().map_err(|e| EngineError::Internal(e.into()))?,
    };
    Ok(Self {
      wasm_engine,
      cache: Arc::new(cache),
      tag_ttl: settings.tag_ttl,
    })
  }

  pub fn try_default() -> Result<Self, EngineError> {
    Self::new(EngineSettings::default())
  }

  fn build_engine() -> Result<wasmtime::Engine, EngineError> {
    let mut config = Config::new();
    #[allow(deprecated)]
    config.async_support(true);
    config.wasm_component_model(true);
    Ok(wasmtime::Engine::new(&config)?)
  }

  pub fn build_store<T>(&self, data: T) -> Store<T> {
    Store::new(&self.wasm_engine, data)
  }

  pub fn load_component(&self, component_bytes: &[u8]) -> Result<Component, EngineError> {
    Component::from_binary(&self.wasm_engine, component_bytes).map_err(|e| EngineError::ComponentLoad(e.to_string()))
  }

  pub fn build_linker<T>(&self) -> Result<Linker<T>, EngineError>
  where
    T: WasiView + WasiHttpView,
  {
    let mut linker: Linker<T> = Linker::new(&self.wasm_engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker)?;
    Ok(linker)
  }

  pub(crate) fn cache(&self) -> &Cache {
    &self.cache
  }

  pub(crate) fn tag_ttl(&self) -> Duration {
    self.tag_ttl
  }
}
