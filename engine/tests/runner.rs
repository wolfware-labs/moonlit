use std::path::Path;
use std::time::Duration;

use moonlit_engine::{Engine, EngineError, EngineSettings, PipelineEvent, PipelineOptions};
use tokio::sync::mpsc::{Receiver, channel};
use tokio_util::sync::CancellationToken;

fn fixture_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_plugin.wasm");
    format!("file://{}", p.display())
}

fn opts() -> PipelineOptions {
    PipelineOptions {
        working_directory: std::env::temp_dir(),
        stages_filter: vec![],
        cli_args: vec![],
        step_timeout: None,
        offline: false,
    }
}

fn drain(rx: &mut Receiver<PipelineEvent>) -> Vec<PipelineEvent> {
    let mut out = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        out.push(ev);
    }
    out
}

async fn load_and_run(
    yaml: &str,
    o: PipelineOptions,
) -> (
    Result<moonlit_engine::PipelineSummary, EngineError>,
    Vec<PipelineEvent>,
) {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, mut rx) = channel(1024);
    let pipeline = eng.load_pipeline(yaml, o, &tx).await.expect("load ok");
    let result = eng.run(pipeline, tx, CancellationToken::new()).await;
    let events = drain(&mut rx);
    (result, events)
}

#[tokio::test(flavor = "multi_thread")]
async fn happy_path_runs_step_and_emits_events() {
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n",
        fixture_url()
    );
    let (result, events) = load_and_run(&yaml, opts()).await;
    let summary = result.expect("run ok");
    assert!(summary.successful);
    assert_eq!(summary.steps.len(), 1);
    assert!(summary.steps[0].successful);

    let has = |pred: &dyn Fn(&PipelineEvent) -> bool| events.iter().any(pred);
    assert!(has(
        &|e| matches!(e, PipelineEvent::StepStarted { name, .. } if name == "s1")
    ));
    assert!(has(
        &|e| matches!(e, PipelineEvent::StepFinished { step, .. } if step == "s1")
    ));
    assert!(has(&|e| matches!(
        e,
        PipelineEvent::PipelineFinished { .. }
    )));
}

#[tokio::test(flavor = "multi_thread")]
async fn falsy_condition_skips_the_step() {
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n      condition: \"false\"\n",
        fixture_url()
    );
    let (result, events) = load_and_run(&yaml, opts()).await;
    let summary = result.expect("run ok");
    assert!(summary.successful);
    assert!(summary.steps[0].skipped);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, PipelineEvent::StepSkipped { step, .. } if step == "s1"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn step_output_is_visible_to_a_later_condition() {
    // s1 (log-and-output) emits output "step" == "s1"; s2's condition reads it.
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n    - name: s2\n      run: tp.log-and-output\n      condition: \"output.s1.step == 's1'\"\n",
        url = fixture_url()
    );
    let (result, _events) = load_and_run(&yaml, opts()).await;
    let summary = result.expect("run ok");
    assert!(summary.successful);
    assert_eq!(summary.steps.len(), 2);
    assert!(
        !summary.steps[1].skipped,
        "s2 condition must resolve true from s1 output"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn failing_step_without_continue_on_error_stops_with_exit_4() {
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: tp.fail\n    - name: s2\n      run: tp.log-and-output\n",
        url = fixture_url()
    );
    let (result, _events) = load_and_run(&yaml, opts()).await;
    let err = match result {
        Ok(_) => panic!("must fail"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 4);
    match err {
        EngineError::Execution(msg) => {
            assert_eq!(
                msg,
                "An error occurred while executing middleware fail: intentional failure"
            )
        }
        other => panic!("expected Execution, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn continue_on_error_runs_past_a_failure_but_reports_unsuccessful() {
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: tp.fail\n      continueOnError: \"true\"\n    - name: s2\n      run: tp.log-and-output\n",
        url = fixture_url()
    );
    let (result, _events) = load_and_run(&yaml, opts()).await;
    let summary = result.expect("run reaches the end");
    assert!(
        !summary.successful,
        "a swallowed failure still marks the pipeline unsuccessful"
    );
    assert_eq!(summary.steps.len(), 2);
    assert!(!summary.steps[0].successful);
    assert!(summary.steps[1].successful);
}

#[tokio::test(flavor = "multi_thread")]
async fn trap_poisons_its_plugin_but_pipeline_continues_on_others() {
    // a.boom traps (continueOnError) -> plugin `a` poisoned for the rest of the run.
    // b.log-and-output runs normally (different plugin). a.log-and-output fails fast (a is dead).
    let url = fixture_url();
    let yaml = format!(
        "name: d\nplugins:\n  - name: a\n    url: {url}\n  - name: b\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: a.boom\n      continueOnError: \"true\"\n    - name: s2\n      run: b.log-and-output\n    - name: s3\n      run: a.log-and-output\n      continueOnError: \"true\"\n"
    );
    let (result, _events) = load_and_run(&yaml, opts()).await;
    let summary = result.expect("run reaches the end (continueOnError past the trap)");
    assert!(!summary.successful);
    assert_eq!(
        summary.steps.len(),
        3,
        "pipeline continued through all three steps"
    );
    assert!(!summary.steps[0].successful, "trapped step recorded failed");
    assert!(
        summary.steps[1].successful,
        "step on the un-poisoned plugin b runs normally"
    );
    assert!(
        !summary.steps[2].successful,
        "step on the poisoned plugin a fails fast"
    );
    assert!(
        summary.steps[2]
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("unavailable after an earlier failure"),
        "poisoned-plugin step carries the clear message"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn non_trap_execute_error_does_not_poison_the_plugin() {
    // bad-output returns successful=true but an invalid-JSON output value -> host BadJson error.
    // The wasm Store is NOT trapped, so a later step on the SAME plugin must still run.
    let url = fixture_url();
    let yaml = format!(
        "name: d\nplugins:\n  - name: a\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: a.bad-output\n      continueOnError: \"true\"\n    - name: s2\n      run: a.log-and-output\n"
    );
    let (result, _events) = load_and_run(&yaml, opts()).await;
    let summary = result.expect("run reaches the end");
    assert!(!summary.successful, "s1 failed (bad JSON output)");
    assert!(!summary.steps[0].successful);
    assert!(
        summary.steps[1].successful,
        "plugin a is NOT poisoned by a non-trap error; s2 runs"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_output_key_fails_the_step() {
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: tp.dup-output\n",
        url = fixture_url()
    );
    let (result, _events) = load_and_run(&yaml, opts()).await;
    let err = match result {
        Ok(_) => panic!("must fail"),
        Err(e) => e,
    };
    match err {
        EngineError::Execution(msg) => assert!(msg.contains("Key 'k' already exists"), "got {msg}"),
        other => panic!("expected Execution, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn halt_if_true_stops_cleanly_and_is_successful() {
    // s1 emits output "step"=="s1"; haltIf true → halt after s1; s2 never runs.
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n      haltIf: \"output.s1.step == 's1'\"\n    - name: s2\n      run: tp.log-and-output\n",
        url = fixture_url()
    );
    let (result, events) = load_and_run(&yaml, opts()).await;
    let summary = result.expect("halt is a success");
    assert!(summary.successful);
    assert!(summary.halted);
    assert_eq!(summary.steps.len(), 1, "s2 must not run after halt");
    assert!(events.iter().any(
        |e| matches!(e, PipelineEvent::PipelineHalted { after_step, .. } if after_step == "s1")
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn halt_if_evaluation_error_fails_the_step() {
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n      haltIf: \"this is not @ valid\"\n",
        url = fixture_url()
    );
    let (result, _events) = load_and_run(&yaml, opts()).await;
    let err = match result {
        Ok(_) => panic!("a broken haltIf must fail the step"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn all_steps_filtered_out_yields_seed_warning_and_success() {
    // stages_filter selects a stage that doesn't exist → zero executable steps.
    let mut o = opts();
    o.stages_filter = vec!["nonexistent".to_string()];
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n",
        url = fixture_url()
    );
    let (result, _events) = load_and_run(&yaml, o).await;
    let summary = result.expect("run ok");
    assert!(summary.successful);
    assert!(summary.steps.is_empty());
    assert_eq!(
        summary.warnings,
        vec!["No middlewares registered in the pipeline.".to_string()]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn step_timeout_is_a_fatal_abort_even_with_continue_on_error() {
    let mut o = opts();
    o.step_timeout = Some(Duration::from_millis(150));
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: slow\n      run: tp.sleep\n      continueOnError: \"true\"\n      config:\n        ms: \"600000\"\n    - name: after\n      run: tp.log-and-output\n",
        url = fixture_url()
    );
    let (result, _events) = load_and_run(&yaml, o).await;
    let err = match result {
        Ok(_) => panic!("timeout must abort"),
        Err(e) => e,
    };
    assert_eq!(err.exit_code(), 4);
    match err {
        EngineError::Execution(msg) => assert!(msg.contains("timed out"), "got {msg}"),
        other => panic!("expected Execution, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_before_a_step_stops_with_the_boundary_message() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(1024);
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n",
        url = fixture_url()
    );
    let pipeline = eng
        .load_pipeline(&yaml, opts(), &tx)
        .await
        .expect("load ok");
    let cancel = CancellationToken::new();
    cancel.cancel(); // already cancelled before the first boundary check
    let err = match eng.run(pipeline, tx, cancel).await {
        Ok(_) => panic!("must stop"),
        Err(e) => e,
    };
    match err {
        EngineError::Execution(msg) => assert_eq!(msg, "Pipeline execution was cancelled."),
        other => panic!("expected Execution, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_during_execute_stops_with_the_user_message() {
    let eng = Engine::new(EngineSettings::default()).unwrap();
    let (tx, _rx) = channel(1024);
    let yaml = format!(
        "name: d\nplugins:\n  - name: tp\n    url: {url}\nstages:\n  build:\n    - name: slow\n      run: tp.sleep\n      config:\n        ms: \"600000\"\n",
        url = fixture_url()
    );
    let pipeline = eng
        .load_pipeline(&yaml, opts(), &tx)
        .await
        .expect("load ok");
    let cancel = CancellationToken::new();
    let canceller = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        canceller.cancel();
    });
    let err = match eng.run(pipeline, tx, cancel).await {
        Ok(_) => panic!("must be cancelled"),
        Err(e) => e,
    };
    match err {
        EngineError::Execution(msg) => {
            assert_eq!(msg, "Pipeline execution was cancelled by the user.")
        }
        other => panic!("expected Execution, got {other:?}"),
    }
}
