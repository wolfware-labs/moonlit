//! Hand-written impls of the custom `moonlit:plugin/host` + `/process` interfaces.
//! wasi:* imports come from wasmtime-wasi and are not implemented here.

use wasmtime::component::Resource;

use crate::host::moonlit::plugin::host::Host as MoonlitHost;
use crate::host::moonlit::plugin::process::{Command, Host as ProcessHost, HostChild, OutputChunk};
use crate::host::moonlit::plugin::types::LogLevel as RawLogLevel;
use crate::host::{ChildProc, HostState};

impl MoonlitHost for HostState {
    async fn log(&mut self, level: RawLogLevel, message: String) -> wasmtime::Result<()> {
        self.events.log(
            &self.current_step,
            crate::host::convert::log_level(level),
            &message,
        );
        Ok(())
    }

    async fn get_config(&mut self, path: String) -> wasmtime::Result<Option<String>> {
        // `:`-separated lookup into the injected config view; serialize the hit as JSON text.
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
        _cmd: Command,
    ) -> wasmtime::Result<Result<Resource<ChildProc>, String>> {
        Ok(Err("process spawn not yet implemented".to_string()))
    }

    async fn run(
        &mut self,
        _cmd: Command,
    ) -> wasmtime::Result<Result<(i32, Vec<OutputChunk>), String>> {
        Ok(Err("process run not yet implemented".to_string()))
    }
}

impl HostChild for HostState {
    async fn next_line(
        &mut self,
        _self_: Resource<ChildProc>,
    ) -> wasmtime::Result<Option<OutputChunk>> {
        Ok(None)
    }

    async fn wait(&mut self, _self_: Resource<ChildProc>) -> wasmtime::Result<i32> {
        Ok(-1)
    }

    async fn kill(&mut self, _self_: Resource<ChildProc>) -> wasmtime::Result<()> {
        Ok(())
    }

    async fn drop(&mut self, rep: Resource<ChildProc>) -> wasmtime::Result<()> {
        let _ = self.table.delete(rep)?;
        Ok(())
    }
}
