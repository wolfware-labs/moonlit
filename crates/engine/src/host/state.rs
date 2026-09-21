use crate::host::HostEventSink;
use crate::host::child_process::ChildProc;
use crate::host::net::AllowlistHooks;
use crate::host::wit::moonlit::plugin::process::{Command, OutputChunk};
use crate::logging::LogLevel;
use std::sync::Arc;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpCtxView, WasiHttpView};

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

impl HostState {
    pub fn new(
        wasi: WasiCtx,
        hooks: AllowlistHooks,
        events: Arc<dyn HostEventSink>,
        config_view: serde_json::Value,
        exec_allow: globset::GlobSet,
    ) -> Self {
        Self {
            table: ResourceTable::new(),
            wasi,
            http: WasiHttpCtx::new(),
            hooks,
            events,
            config_view,
            exec_allow,
            current_step: String::new(),
        }
    }
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

impl MoonlitHost for HostState {
    async fn log(&mut self, level: LogLevel, message: String) -> wasmtime::Result<()> {
        self.events.log(&self.current_step, level.into(), &message);
        Ok(())
    }

    async fn get_config(&mut self, path: String) -> wasmtime::Result<Option<String>> {
        let mut cur = &self.config_view;
        for seg in path.split(':') {
            match cur.get(seg) {
                Some(next) => cur = next,
                None => return Ok(None),
            }
        }
        Ok(Some(cur.to_string()))
    }

    async fn report_progress(&mut self, message: String) -> wasmtime::Result<()> {
        self.events.progress(&self.current_step, &message);
        Ok(())
    }
}

impl ProcessHost for HostState {
    async fn spawn(
        &mut self,
        cmd: Command,
    ) -> wasmtime::Result<Result<Resource<ChildProc>, String>> {
        if !self.exec_allow.is_match(&cmd.program) {
            self.events.log(
                &self.current_step,
                crate::host::LogLevel::Warn,
                &format!(
                    "blocked from running '{}' — add it to permissions.exec",
                    cmd.program
                ),
            );
            return Ok(Err(format!("program '{}' not permitted", cmd.program)));
        }
        match ChildProc::start(&cmd) {
            Ok(child) => Ok(Ok(self.table.push(child)?)),
            Err(e) => Ok(Err(e)),
        }
    }

    async fn run(
        &mut self,
        cmd: Command,
    ) -> wasmtime::Result<Result<(i32, Vec<OutputChunk>), String>> {
        if !self.exec_allow.is_match(&cmd.program) {
            self.events.log(
                &self.current_step,
                crate::host::LogLevel::Warn,
                &format!(
                    "blocked from running '{}' — add it to permissions.exec",
                    cmd.program
                ),
            );
            return Ok(Err(format!("program '{}' not permitted", cmd.program)));
        }
        let mut child = match ChildProc::start(&cmd) {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        let mut chunks = Vec::new();
        while let Some(line) = child.rx.recv().await {
            chunks.push(line);
        }
        let code = match child.exit_rx.take() {
            Some(rx) => rx.await.unwrap_or(-1),
            None => -1,
        };
        Ok(Ok((code, chunks)))
    }
}

impl HostChild for HostState {
    async fn next_line(
        &mut self,
        self_: Resource<ChildProc>,
    ) -> wasmtime::Result<Option<OutputChunk>> {
        let child = self.table.get_mut(&self_)?;
        Ok(child.rx.recv().await)
    }

    async fn wait(&mut self, self_: Resource<ChildProc>) -> wasmtime::Result<i32> {
        let child = self.table.get_mut(&self_)?;
        if let Some(code) = child.exit_cached {
            return Ok(code);
        }
        let code = match child.exit_rx.take() {
            Some(rx) => rx.await.unwrap_or(-1),
            None => -1,
        };
        child.exit_cached = Some(code);
        Ok(code)
    }

    async fn kill(&mut self, self_: Resource<ChildProc>) -> wasmtime::Result<()> {
        let child = self.table.get_mut(&self_)?;
        if let Some(tx) = child.kill_tx.take() {
            let _ = tx.send(());
        }
        Ok(())
    }

    async fn drop(&mut self, rep: Resource<ChildProc>) -> wasmtime::Result<()> {
        let _ = self.table.delete(rep)?;
        Ok(())
    }
}
