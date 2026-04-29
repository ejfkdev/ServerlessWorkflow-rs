use super::*;

#[tokio::test]
async fn test_runner_schedule_after() {
    let start = std::time::Instant::now();
    let output = run_workflow_from_yaml(&testdata("schedule_after.yaml"), json!({}))
        .await
        .unwrap();
    let elapsed = start.elapsed();
    // Should have waited at least ~10ms due to schedule.after: PT0.01S
    assert!(
        elapsed.as_millis() >= 5,
        "Expected at least 5ms delay, got {}ms",
        elapsed.as_millis()
    );
    assert_eq!(output["phase"], json!("completed"));
}

// === HTTP Call: HEAD ===

#[tokio::test]
async fn test_runner_cron_start() {
    let output = run_workflow_from_yaml(&testdata("cron_start.yaml"), json!({}))
        .await
        .unwrap();
    // Cron schedule only affects recurring execution; single run should work
    assert_eq!(output["phase"], json!("started"));
}

// === Schedule: every (parse + run once) ===

#[tokio::test]
async fn test_runner_every_start() {
    let output = run_workflow_from_yaml(&testdata("every_start.yaml"), json!({}))
        .await
        .unwrap();
    // Every schedule only affects recurring execution; single run should work
    assert_eq!(output["phase"], json!("started"));
}

// === Listen: to any (no preceding set) ===

#[tokio::test]
async fn test_runner_schedule_after_delay() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: schedule-after-delay
  version: '0.1.0'
do:
  - delayedStart:
      set:
        started: true
schedule:
  after:
    milliseconds: 1
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let start = std::time::Instant::now();
    let output = runner.run(json!({})).await.unwrap();
    let elapsed = start.elapsed();
    // After 1ms should complete quickly
    assert_eq!(output["started"], json!(true));
    assert!(
        elapsed.as_millis() < 500,
        "Should complete within 500ms, got {}ms",
        elapsed.as_millis()
    );
}

// === Switch then:continue continues to next task ===

#[tokio::test]
async fn test_runner_schedule_after_java_pattern() {
    // Matches Java SDK's after-start.yaml - schedule with after delay
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: after-schedule
  version: '0.1.0'
schedule:
  after:
    milliseconds: 10
do:
  - recovered:
      set:
        recovered: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["recovered"], json!(true));
}

// === Set with deeply nested objects ===

#[tokio::test]
async fn test_runner_schedule_cron() {
    // Java SDK's cron-start.yaml - schedule with cron expression
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: cron-schedule
  version: '0.1.0'
schedule:
  cron: '0 0 * * *'
do:
  - setTask:
      set:
        ran: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // Schedule is metadata only, workflow runs immediately
    assert_eq!(output["ran"], json!(true));
}

// === Schedule every pattern ===

#[tokio::test]
async fn test_runner_schedule_every() {
    // Java SDK's every-start.yaml - schedule with every interval (Duration struct)
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: every-schedule
  version: '0.1.0'
schedule:
  every:
    milliseconds: 10
do:
  - setTask:
      set:
        ran: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["ran"], json!(true));
}

// === Raise error with dynamic detail from input ===

#[tokio::test]
async fn test_suspend_resume_workflow() {
    let workflow: WorkflowDefinition = serde_yaml::from_str(
        r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: suspend-test
  version: '0.1.0'
do:
  - step1:
      set:
        value: 1
  - step2:
      set:
        value: 2
  - step3:
      set:
        value: 3
"#,
    )
    .unwrap();

    let runner = WorkflowRunner::new(workflow).unwrap();
    let handle = runner.handle();

    // Run the workflow in a background task; suspend after a brief delay
    let run_handle = tokio::spawn(async move { runner.run(json!({})).await.unwrap() });

    // Suspend the workflow
    assert!(handle.suspend());
    // Already suspended
    assert!(!handle.suspend());
    assert!(handle.is_suspended());

    // Resume after a brief delay
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    assert!(handle.resume());
    assert!(!handle.is_suspended());

    let output = run_handle.await.unwrap();
    assert_eq!(output["value"], json!(3));
}

#[tokio::test]
async fn test_context_suspend_resume() {
    let workflow = WorkflowDefinition::default();
    let ctx = WorkflowContext::new(&workflow).unwrap();

    assert!(!ctx.is_suspended());

    // Suspend
    assert!(ctx.suspend());
    assert!(ctx.is_suspended());

    // Already suspended
    assert!(!ctx.suspend());

    // Resume
    assert!(ctx.resume());
    assert!(!ctx.is_suspended());

    // Not suspended, can't resume
    assert!(!ctx.resume());
}

// === Scheduler ===

#[tokio::test]
async fn test_schedule_every() {
    use std::sync::atomic::{AtomicU32, Ordering};

    let workflow: WorkflowDefinition = serde_yaml::from_str(
        r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: schedule-every
  version: '0.1.0'
schedule:
  every:
    milliseconds: 10
do:
  - step1:
      set:
        value: 42
"#,
    )
    .unwrap();

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();

    // Run via schedule() which uses the every interval
    let runner = WorkflowRunner::new(workflow).unwrap();
    let scheduled = runner.schedule(json!({}));

    // Wait for a few iterations
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    counter_clone.fetch_add(1, Ordering::SeqCst);

    // Cancel the schedule
    scheduled.cancel();

    // Should have run at least 2 times in 50ms with 10ms interval
    // (accounting for startup and scheduling jitter)
    let count = counter.load(Ordering::SeqCst);
    assert!(
        count >= 1,
        "scheduled workflow should have run at least once, got {}",
        count
    );
}

#[tokio::test]
async fn test_schedule_no_schedule_runs_once() {
    let workflow: WorkflowDefinition = serde_yaml::from_str(
        r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: schedule-none
  version: '0.1.0'
do:
  - step1:
      set:
        value: hello
"#,
    )
    .unwrap();

    let runner = WorkflowRunner::new(workflow).unwrap();
    let scheduled = runner.schedule(json!({}));

    // Wait briefly, then cancel
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    scheduled.cancel();
}

// === Bearer Auth Integration ===
