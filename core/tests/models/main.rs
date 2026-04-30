use serde_json::json;
use swf_core::models::authentication::*;
use swf_core::models::duration::*;
use swf_core::models::error::*;
use swf_core::models::extension::ExtensionDefinition;
use swf_core::models::map::*;
use swf_core::models::resource::OneOfEndpointDefinitionOrUri;
use swf_core::models::retry::*;
use swf_core::models::task::*;
use swf_core::models::timeout::*;
use swf_core::models::workflow::*;

#[test]
fn create_workflow() {
    let namespace = "fake-namespace";
    let name = "fake-workflow";
    let version = "1.0.0";
    let title = Some("fake-title".to_string());
    let summary = Some("fake-summary".to_string());
    let document = WorkflowDefinitionMetadata::new(
        namespace,
        name,
        version,
        title.clone(),
        summary.clone(),
        None,
    );
    let call_task = CallTaskDefinition::Function(
        swf_core::models::call::CallFunctionDefinition {
            call: "http".to_string(),
            with: None,
            common: swf_core::models::task::TaskDefinitionFields::default(),
        },
    );
    let do_task = DoTaskDefinition::new(Map::from(vec![(
        "set".to_string(),
        TaskDefinition::Wait(WaitTaskDefinition::new(
            OneOfDurationOrIso8601Expression::Duration(Duration::from_milliseconds(200)),
        )),
    )]));
    let mut workflow = WorkflowDefinition::new(document);
    workflow.do_ = Map::new();
    workflow.do_.add(
        "callTask".to_string(),
        TaskDefinition::Call(Box::new(call_task)),
    );
    workflow
        .do_
        .add("doTask".to_string(), TaskDefinition::Do(do_task));
    let json_serialization_result = serde_json::to_string_pretty(&workflow);
    let yaml_serialization_result = serde_yaml::to_string(&workflow);
    assert!(
        json_serialization_result.is_ok(),
        "JSON Serialization failed: {:?}",
        json_serialization_result.err()
    );
    assert!(
        yaml_serialization_result.is_ok(),
        "YAML Serialization failed: {:?}",
        yaml_serialization_result.err()
    );
    if let Result::Ok(yaml) = yaml_serialization_result {
        println!("{}", yaml)
    }
    assert_eq!(workflow.document.namespace, namespace);
    assert_eq!(workflow.document.name, name);
    assert_eq!(workflow.document.version, version);
    assert_eq!(workflow.document.title, title);
    assert_eq!(workflow.document.summary, summary);
}

mod auth;
mod call;
mod do_task;
mod emit;
mod error;
mod event;
mod extension;
mod for_loop;
mod fork;
mod listen;
mod misc;
mod raise;
mod retry;
mod run;
mod set;
mod switch;
mod timeout;
mod wait;
mod workflow;
