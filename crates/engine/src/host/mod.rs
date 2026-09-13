mod convert;
mod imports;
mod net;
mod perms;

use std::path::PathBuf;
use std::sync::Arc;

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::WasiHttpCtx;
use wasmtime_wasi_http::{WasiHttpCtxView, WasiHttpView};

use net::AllowlistHooks;

pub use convert::{HostError, json_str_to_value, json_to_value, value_to_json};
pub use convert::{LogLevel, MiddlewareInfo, MiddlewareResult, PluginMetadata, ReleaseContext};

pub struct InstanceConfig {
    pub working_directory: PathBuf,
    pub permissions: crate::config::model::Permissions,
    pub config_view: serde_json::Value,
    pub env_snapshot: Vec<(String, String)>,
}

mod raw {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "plugin-host",
        imports: { default: async | trappable },
        exports: { default: async },
        with: {
            "wasi": wasmtime_wasi::p2::bindings,
            "moonlit:plugin/process.child": crate::host::ChildProc,
        },
    });
}

use raw::PluginHost;
pub(crate) use raw::moonlit;

pub trait HostEventSink: Send + Sync {
    fn log(&self, step: &str, level: LogLevel, message: &str);
    fn progress(&self, step: &str, message: &str);
}

pub struct ChildProc {
    rx: tokio::sync::mpsc::Receiver<moonlit::plugin::process::OutputChunk>,
    exit_rx: Option<tokio::sync::oneshot::Receiver<i32>>,
    exit_cached: Option<i32>,
    kill_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

pub struct HostState {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
    hooks: AllowlistHooks,
    events: Arc<dyn HostEventSink>,
    config_view: serde_json::Value,
    exec_allow: globset::GlobSet,
    current_step: String,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for HostState {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: &mut self.hooks,
        }
    }
}

pub(crate) fn build_engine() -> anyhow::Result<Engine> {
    let mut config = Config::new();
    #[allow(deprecated)]
    config.async_support(true);
    config.wasm_component_model(true);
    Ok(Engine::new(&config)?)
}

fn build_linker(engine: &Engine) -> anyhow::Result<Linker<HostState>> {
    let mut linker: Linker<HostState> = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker)?;
    moonlit::plugin::host::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
    moonlit::plugin::process::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
    Ok(linker)
}

pub fn test_engine() -> Engine {
    build_engine().expect("engine build")
}

pub struct PluginInstance {
    store: Store<HostState>,
    bindings: PluginHost,
}

impl PluginInstance {
    pub async fn instantiate(
        engine: &Engine,
        component_bytes: &[u8],
        cfg: InstanceConfig,
        events: Arc<dyn HostEventSink>,
    ) -> Result<PluginInstance, HostError> {
        let linker = build_linker(engine).map_err(|e| HostError::Link(e.to_string()))?;
        let wasi =
            perms::build_wasi_ctx(&cfg).map_err(|e| HostError::Instantiate(e.to_string()))?;
        events.log(
            "",
            LogLevel::Debug,
            &format!(
                "plugin grants — network={:?} exec={:?} env={:?} filesystem={:?}",
                cfg.permissions.network,
                cfg.permissions.exec,
                cfg.permissions.env,
                cfg.permissions.filesystem
            ),
        );
        let state = HostState {
            table: ResourceTable::new(),
            wasi,
            http: WasiHttpCtx::new(),
            hooks: AllowlistHooks::new(&cfg.permissions, events.clone()),
            events,
            config_view: cfg.config_view,
            exec_allow: perms::exec_globset(&cfg.permissions.exec),
            current_step: String::new(),
        };
        let mut store = Store::new(engine, state);
        let component = Component::from_binary(engine, component_bytes)
            .map_err(|e| HostError::Instantiate(e.to_string()))?;
        let bindings = PluginHost::instantiate_async(&mut store, &component, &linker)
            .await
            .map_err(|e| HostError::Instantiate(e.to_string()))?;
        Ok(PluginInstance { store, bindings })
    }

    pub async fn describe(&mut self) -> Result<PluginMetadata, HostError> {
        match self.bindings.call_describe(&mut self.store).await {
            Ok(meta) => Ok(convert::metadata(meta)),
            Err(e) => Err(HostError::Trap {
                op: "describe".to_string(),
                message: format!("{e:?}"),
            }),
        }
    }

    pub async fn init(
        &mut self,
        plugin_config: &serde_json::Value,
    ) -> Result<PluginMetadata, String> {
        self.store.data_mut().current_step = "init".to_string();
        let json = plugin_config.to_string();
        match self.bindings.call_init(&mut self.store, &json).await {
            Ok(Ok(meta)) => Ok(convert::metadata(meta)),
            Ok(Err(msg)) => Err(msg),
            Err(trap) => Err(format!("plugin trapped during init: {trap:?}")),
        }
    }

    pub async fn execute(
        &mut self,
        middleware: &str,
        ctx: ReleaseContext,
        config: &serde_json::Value,
    ) -> Result<MiddlewareResult, HostError> {
        self.store.data_mut().current_step = ctx.step_name.clone();
        let raw_ctx = convert::release_context_to_raw(&ctx);
        let json = config.to_string();
        match self
            .bindings
            .call_execute(&mut self.store, middleware, &raw_ctx, &json)
            .await
        {
            Ok(raw) => convert::middleware_result(raw),
            Err(e) => Err(HostError::Trap {
                op: format!("execute {middleware}"),
                message: format!("{e:?}"),
            }),
        }
    }

    pub async fn list_middlewares(&mut self) -> Result<Vec<MiddlewareInfo>, HostError> {
        match self.bindings.call_list_middlewares(&mut self.store).await {
            Ok(list) => Ok(list.into_iter().map(convert::middleware_info).collect()),
            Err(e) => Err(HostError::Trap {
                op: "list-middlewares".to_string(),
                message: format!("{e:?}"),
            }),
        }
    }
}
