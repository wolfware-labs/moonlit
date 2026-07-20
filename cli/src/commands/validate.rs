//! `moonlit validate` — load and check a pipeline without executing it (delegates to the
//! run orchestration with `load_only = true`).

use crate::cli::{OutputMode, RunArgs, ValidateArgs};

pub async fn run(output: Option<OutputMode>, verbose: bool, args: ValidateArgs) -> i32 {
    let run_args = RunArgs {
        file: args.file,
        working_dir: args.working_dir,
        stages: vec![],
        args: vec![],
        offline: false,
        step_timeout: None,
        dry_run: true,
    };
    let code = super::run::run(output, verbose, run_args, true).await;
    if code == 0 {
        eprintln!("✔ Configuration valid");
    }
    code
}
