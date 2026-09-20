use std::sync::Arc;

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
