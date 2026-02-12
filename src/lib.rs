#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

mod asset_resolver;
mod error;
mod expression;
mod interaction;
mod model;
mod render;
mod state_store;
mod trace;
mod validation;

use std::collections::{BTreeMap, BTreeSet};

use greentic_types::cbor::canonical;
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};
use greentic_types::schemas::component::v0_6_0::{
    ComponentDescribe, ComponentInfo, ComponentOperation, ComponentRunInput, ComponentRunOutput,
    I18nText, schema_hash,
};
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::component::v0_6_0::{
    ComponentQaSpec, QaMode as ComponentQaMode, Question, QuestionKind,
};
use once_cell::sync::Lazy;

pub use asset_resolver::{
    register_host_asset_callback, register_host_asset_map, register_host_asset_resolver,
};
pub use error::ComponentError;
pub use interaction::handle_interaction;
pub use model::*;
pub use render::render_card;

const COMPONENT_NAME: &str = "component-adaptive-card";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = "0.1.12";
const COMPONENT_ID: &str = "ai.greentic.component-adaptive-card";
const COMPONENT_ROLE: &str = "tool";

static COMPONENT_SCHEMA_JSON: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::from_str(include_str!("../schemas/component.schema.json"))
        .expect("failed to parse component schema")
});
static INPUT_SCHEMA_JSON: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::from_str(include_str!("../schemas/io/input.schema.json"))
        .expect("failed to parse input schema")
});
static OUTPUT_SCHEMA_JSON: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::from_str(include_str!("../schemas/io/output.schema.json"))
        .expect("failed to parse output schema")
});

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

#[cfg(target_arch = "wasm32")]
mod component {
    use greentic_interfaces_guest::component_v0_6::{
        component_descriptor, component_i18n, component_qa, component_runtime, component_schema,
    };

    use super::{
        apply_answers_cbor, component_describe_cbor, component_info_cbor, config_schema_cbor,
        i18n_keys, input_schema_cbor, output_schema_cbor, qa_spec_cbor, run_component_cbor,
    };

    pub(super) struct Component;

    impl component_descriptor::Guest for Component {
        fn get_component_info() -> Vec<u8> {
            component_info_cbor()
        }

        fn describe() -> Vec<u8> {
            component_describe_cbor()
        }
    }

    impl component_schema::Guest for Component {
        fn input_schema() -> Vec<u8> {
            input_schema_cbor()
        }

        fn output_schema() -> Vec<u8> {
            output_schema_cbor()
        }

        fn config_schema() -> Vec<u8> {
            config_schema_cbor()
        }
    }

    impl component_runtime::Guest for Component {
        fn run(input: Vec<u8>, state: Vec<u8>) -> component_runtime::RunResult {
            let (output, new_state) = run_component_cbor(input, state);
            component_runtime::RunResult { output, new_state }
        }
    }

    impl component_qa::Guest for Component {
        fn qa_spec(mode: component_qa::QaMode) -> Vec<u8> {
            qa_spec_cbor(mode)
        }

        fn apply_answers(
            mode: component_qa::QaMode,
            current_config: Vec<u8>,
            answers: Vec<u8>,
        ) -> Vec<u8> {
            apply_answers_cbor(mode, current_config, answers)
        }
    }

    impl component_i18n::Guest for Component {
        fn i18n_keys() -> Vec<String> {
            i18n_keys()
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod legacy_component_v0_5 {
    use greentic_interfaces_guest::component::node::{
        self, ExecCtx, InvokeResult, LifecycleStatus, StreamEvent,
    };

    use super::{describe_payload, handle_message};

    pub(super) struct ComponentV05;

    impl node::Guest for ComponentV05 {
        fn get_manifest() -> String {
            describe_payload()
        }

        fn on_start(_ctx: ExecCtx) -> Result<LifecycleStatus, String> {
            Ok(LifecycleStatus::Ok)
        }

        fn on_stop(_ctx: ExecCtx, _reason: String) -> Result<LifecycleStatus, String> {
            Ok(LifecycleStatus::Ok)
        }

        fn invoke(_ctx: ExecCtx, op: String, input: String) -> InvokeResult {
            InvokeResult::Ok(handle_message(&op, &input))
        }

        fn invoke_stream(_ctx: ExecCtx, op: String, input: String) -> Vec<StreamEvent> {
            vec![
                StreamEvent::Progress(0),
                StreamEvent::Data(handle_message(&op, &input)),
                StreamEvent::Done,
            ]
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod exports {
    use greentic_interfaces_guest::component_v0_6::{
        component_descriptor, component_i18n, component_qa, component_runtime, component_schema,
    };

    use super::component::Component;

    #[unsafe(export_name = "greentic:component/component-descriptor@0.6.0#get-component-info")]
    unsafe extern "C" fn export_get_component_info() -> *mut u8 {
        unsafe { component_descriptor::_export_get_component_info_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-descriptor@0.6.0#get-component-info")]
    unsafe extern "C" fn post_return_get_component_info(arg0: *mut u8) {
        unsafe { component_descriptor::__post_return_get_component_info::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-descriptor@0.6.0#describe")]
    unsafe extern "C" fn export_describe() -> *mut u8 {
        unsafe { component_descriptor::_export_describe_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-descriptor@0.6.0#describe")]
    unsafe extern "C" fn post_return_describe(arg0: *mut u8) {
        unsafe { component_descriptor::__post_return_describe::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-schema@0.6.0#input-schema")]
    unsafe extern "C" fn export_input_schema() -> *mut u8 {
        unsafe { component_schema::_export_input_schema_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-schema@0.6.0#input-schema")]
    unsafe extern "C" fn post_return_input_schema(arg0: *mut u8) {
        unsafe { component_schema::__post_return_input_schema::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-schema@0.6.0#output-schema")]
    unsafe extern "C" fn export_output_schema() -> *mut u8 {
        unsafe { component_schema::_export_output_schema_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-schema@0.6.0#output-schema")]
    unsafe extern "C" fn post_return_output_schema(arg0: *mut u8) {
        unsafe { component_schema::__post_return_output_schema::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-schema@0.6.0#config-schema")]
    unsafe extern "C" fn export_config_schema() -> *mut u8 {
        unsafe { component_schema::_export_config_schema_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-schema@0.6.0#config-schema")]
    unsafe extern "C" fn post_return_config_schema(arg0: *mut u8) {
        unsafe { component_schema::__post_return_config_schema::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-runtime@0.6.0#run")]
    unsafe extern "C" fn export_run(
        arg0: *mut u8,
        arg1: usize,
        arg2: *mut u8,
        arg3: usize,
    ) -> *mut u8 {
        unsafe { component_runtime::_export_run_cabi::<Component>(arg0, arg1, arg2, arg3) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-runtime@0.6.0#run")]
    unsafe extern "C" fn post_return_run(arg0: *mut u8) {
        unsafe { component_runtime::__post_return_run::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-qa@0.6.0#qa-spec")]
    unsafe extern "C" fn export_qa_spec(arg0: i32) -> *mut u8 {
        unsafe { component_qa::_export_qa_spec_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-qa@0.6.0#qa-spec")]
    unsafe extern "C" fn post_return_qa_spec(arg0: *mut u8) {
        unsafe { component_qa::__post_return_qa_spec::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-qa@0.6.0#apply-answers")]
    unsafe extern "C" fn export_apply_answers(
        arg0: i32,
        arg1: *mut u8,
        arg2: usize,
        arg3: *mut u8,
        arg4: usize,
    ) -> *mut u8 {
        unsafe {
            component_qa::_export_apply_answers_cabi::<Component>(arg0, arg1, arg2, arg3, arg4)
        }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-qa@0.6.0#apply-answers")]
    unsafe extern "C" fn post_return_apply_answers(arg0: *mut u8) {
        unsafe { component_qa::__post_return_apply_answers::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-i18n@0.6.0#i18n-keys")]
    unsafe extern "C" fn export_i18n_keys() -> *mut u8 {
        unsafe { component_i18n::_export_i18n_keys_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-i18n@0.6.0#i18n-keys")]
    unsafe extern "C" fn post_return_i18n_keys(arg0: *mut u8) {
        unsafe { component_i18n::__post_return_i18n_keys::<Component>(arg0) };
    }
}

#[cfg(target_arch = "wasm32")]
mod legacy_exports_v0_5 {
    use greentic_interfaces_guest::component::node;

    use super::legacy_component_v0_5::ComponentV05;

    #[unsafe(export_name = "greentic:component/node@0.5.0#get-manifest")]
    unsafe extern "C" fn export_get_manifest() -> *mut u8 {
        unsafe { node::_export_get_manifest_cabi::<ComponentV05>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#get-manifest")]
    unsafe extern "C" fn post_return_get_manifest(arg0: *mut u8) {
        unsafe { node::__post_return_get_manifest::<ComponentV05>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#on-start")]
    unsafe extern "C" fn export_on_start(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_on_start_cabi::<ComponentV05>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#on-start")]
    unsafe extern "C" fn post_return_on_start(arg0: *mut u8) {
        unsafe { node::__post_return_on_start::<ComponentV05>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#on-stop")]
    unsafe extern "C" fn export_on_stop(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_on_stop_cabi::<ComponentV05>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#on-stop")]
    unsafe extern "C" fn post_return_on_stop(arg0: *mut u8) {
        unsafe { node::__post_return_on_stop::<ComponentV05>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#invoke")]
    unsafe extern "C" fn export_invoke(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_invoke_cabi::<ComponentV05>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#invoke")]
    unsafe extern "C" fn post_return_invoke(arg0: *mut u8) {
        unsafe { node::__post_return_invoke::<ComponentV05>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#invoke-stream")]
    unsafe extern "C" fn export_invoke_stream(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_invoke_stream_cabi::<ComponentV05>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#invoke-stream")]
    unsafe extern "C" fn post_return_invoke_stream(arg0: *mut u8) {
        unsafe { node::__post_return_invoke_stream::<ComponentV05>(arg0) };
    }
}

pub fn describe_payload() -> String {
    serde_json::json!({
        "component": {
            "name": COMPONENT_NAME,
            "org": COMPONENT_ORG,
            "version": COMPONENT_VERSION,
            "world": "greentic:component/component@0.6.0",
            "schemas": {
                "component": COMPONENT_SCHEMA_JSON.clone(),
                "input": INPUT_SCHEMA_JSON.clone(),
                "output": OUTPUT_SCHEMA_JSON.clone()
            }
        }
    })
    .to_string()
}

fn encode_cbor<T: serde::Serialize>(value: &T) -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(value).expect("encode cbor")
}

fn decode_cbor<T: for<'de> serde::Deserialize<'de>>(bytes: &[u8]) -> Result<T, ComponentError> {
    canonical::from_cbor(bytes)
        .map_err(|err| ComponentError::InvalidInput(format!("failed to decode cbor: {err}")))
}

fn schema_from_json(value: &serde_json::Value) -> SchemaIr {
    if let Some(one_of) = value.get("oneOf").and_then(|v| v.as_array()) {
        return SchemaIr::OneOf {
            variants: one_of.iter().map(schema_from_json).collect(),
        };
    }
    if let Some(any_of) = value.get("anyOf").and_then(|v| v.as_array()) {
        return SchemaIr::OneOf {
            variants: any_of.iter().map(schema_from_json).collect(),
        };
    }
    if let Some(types) = value.get("type").and_then(|v| v.as_array()) {
        return SchemaIr::OneOf {
            variants: types
                .iter()
                .map(|entry| schema_from_json(&serde_json::json!({ "type": entry })))
                .collect(),
        };
    }

    match value.get("type").and_then(|v| v.as_str()) {
        Some("object") => {
            let properties = value
                .get("properties")
                .and_then(|v| v.as_object())
                .map(|props| {
                    props
                        .iter()
                        .map(|(name, schema)| (name.clone(), schema_from_json(schema)))
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            let required = value
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|entry| entry.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let additional = match value.get("additionalProperties") {
                Some(serde_json::Value::Bool(false)) => AdditionalProperties::Forbid,
                Some(serde_json::Value::Object(obj)) => AdditionalProperties::Schema(Box::new(
                    schema_from_json(&serde_json::Value::Object(obj.clone())),
                )),
                _ => AdditionalProperties::Allow,
            };
            SchemaIr::Object {
                properties,
                required,
                additional,
            }
        }
        Some("array") => {
            let items =
                value
                    .get("items")
                    .map(schema_from_json)
                    .unwrap_or_else(|| SchemaIr::String {
                        min_len: Some(0),
                        max_len: None,
                        regex: None,
                        format: None,
                    });
            SchemaIr::Array {
                items: Box::new(items),
                min_items: value.get("minItems").and_then(|v| v.as_u64()),
                max_items: value.get("maxItems").and_then(|v| v.as_u64()),
            }
        }
        Some("string") => SchemaIr::String {
            min_len: value.get("minLength").and_then(|v| v.as_u64()),
            max_len: value.get("maxLength").and_then(|v| v.as_u64()),
            regex: value
                .get("pattern")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned),
            format: value
                .get("format")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned),
        },
        Some("integer") => SchemaIr::Int {
            min: value.get("minimum").and_then(|v| v.as_i64()),
            max: value.get("maximum").and_then(|v| v.as_i64()),
        },
        Some("number") => SchemaIr::Float {
            min: value.get("minimum").and_then(|v| v.as_f64()),
            max: value.get("maximum").and_then(|v| v.as_f64()),
        },
        Some("boolean") => SchemaIr::Bool,
        Some("null") => SchemaIr::Null,
        _ => SchemaIr::String {
            min_len: Some(0),
            max_len: None,
            regex: None,
            format: None,
        },
    }
}

fn component_info() -> ComponentInfo {
    ComponentInfo {
        id: COMPONENT_ID.to_string(),
        version: COMPONENT_VERSION.to_string(),
        role: COMPONENT_ROLE.to_string(),
        display_name: Some(I18nText::new(
            "adaptive_card.display_name",
            Some("Adaptive Card".to_string()),
        )),
    }
}

fn input_schema_ir() -> SchemaIr {
    schema_from_json(&INPUT_SCHEMA_JSON)
}

fn output_schema_ir() -> SchemaIr {
    schema_from_json(&OUTPUT_SCHEMA_JSON)
}

fn config_schema_ir() -> SchemaIr {
    schema_from_json(&COMPONENT_SCHEMA_JSON)
}

fn component_describe() -> ComponentDescribe {
    let input = input_schema_ir();
    let output = output_schema_ir();
    let config = config_schema_ir();
    let hash = schema_hash(&input, &output, &config).unwrap_or_default();
    ComponentDescribe {
        info: component_info(),
        provided_capabilities: Vec::new(),
        required_capabilities: Vec::new(),
        metadata: BTreeMap::new(),
        operations: vec![ComponentOperation {
            id: "card".to_string(),
            display_name: Some(I18nText::new(
                "adaptive_card.operation.card",
                Some("Render adaptive card".to_string()),
            )),
            input: ComponentRunInput { schema: input },
            output: ComponentRunOutput { schema: output },
            defaults: BTreeMap::new(),
            redactions: Vec::new(),
            constraints: BTreeMap::new(),
            schema_hash: hash,
        }],
        config_schema: config,
    }
}

fn component_info_cbor() -> Vec<u8> {
    encode_cbor(&component_info())
}

fn component_describe_cbor() -> Vec<u8> {
    encode_cbor(&component_describe())
}

fn input_schema_cbor() -> Vec<u8> {
    encode_cbor(&input_schema_ir())
}

fn output_schema_cbor() -> Vec<u8> {
    encode_cbor(&output_schema_ir())
}

fn config_schema_cbor() -> Vec<u8> {
    encode_cbor(&config_schema_ir())
}

#[cfg(target_arch = "wasm32")]
fn qa_spec_cbor(mode: greentic_interfaces_guest::component_v0_6::component_qa::QaMode) -> Vec<u8> {
    let mode = match mode {
        greentic_interfaces_guest::component_v0_6::component_qa::QaMode::Default => {
            ComponentQaMode::Default
        }
        greentic_interfaces_guest::component_v0_6::component_qa::QaMode::Setup => {
            ComponentQaMode::Setup
        }
        greentic_interfaces_guest::component_v0_6::component_qa::QaMode::Upgrade => {
            ComponentQaMode::Upgrade
        }
        greentic_interfaces_guest::component_v0_6::component_qa::QaMode::Remove => {
            ComponentQaMode::Remove
        }
    };
    let spec = ComponentQaSpec {
        mode,
        title: I18nText::new(
            "adaptive_card.qa.title",
            Some("Adaptive Card settings".to_string()),
        ),
        description: None,
        questions: vec![Question {
            id: "card_source".to_string(),
            label: I18nText::new(
                "adaptive_card.qa.card_source",
                Some("Card source".to_string()),
            ),
            help: None,
            error: None,
            kind: QuestionKind::Text,
            required: false,
            default: None,
        }],
        defaults: BTreeMap::new(),
    };
    encode_cbor(&spec)
}

#[cfg(target_arch = "wasm32")]
fn apply_answers_cbor(
    _mode: greentic_interfaces_guest::component_v0_6::component_qa::QaMode,
    current_config: Vec<u8>,
    answers: Vec<u8>,
) -> Vec<u8> {
    let current: Result<serde_json::Value, _> = canonical::from_cbor(&current_config);
    let incoming: Result<serde_json::Value, _> = canonical::from_cbor(&answers);
    let merged = match (current.ok(), incoming.ok()) {
        (_, Some(value @ serde_json::Value::Object(_))) => value,
        (Some(value @ serde_json::Value::Object(_)), _) => value,
        _ => serde_json::json!({}),
    };
    encode_cbor(&merged)
}

fn i18n_keys() -> Vec<String> {
    let mut keys = BTreeSet::new();
    keys.insert("adaptive_card.qa.title".to_string());
    keys.insert("adaptive_card.qa.card_source".to_string());
    keys.into_iter().collect()
}

fn run_component_cbor(input: Vec<u8>, _state: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    let input_json: Result<serde_json::Value, _> = decode_cbor(&input);
    let output_json = match input_json {
        Ok(value) => {
            let op = value
                .get("operation")
                .and_then(|v| v.as_str())
                .unwrap_or("card");
            let raw = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
            handle_message(op, &raw)
        }
        Err(err) => error_payload(
            "AC_SCHEMA_INVALID",
            "invalid cbor invocation",
            Some(serde_json::Value::String(err.to_string())),
        ),
    };
    let parsed: serde_json::Value =
        serde_json::from_str(&output_json).unwrap_or_else(|_| serde_json::json!({}));
    (encode_cbor(&parsed), encode_cbor(&serde_json::json!({})))
}

pub fn handle_message(operation: &str, input: &str) -> String {
    let value: serde_json::Value = match serde_json::from_str(input) {
        Ok(value) => value,
        Err(err) => {
            return error_payload(
                "AC_SCHEMA_INVALID",
                "invalid JSON",
                Some(serde_json::Value::String(err.to_string())),
            );
        }
    };
    let invocation_value =
        validation::locate_invocation_candidate(&value).unwrap_or_else(|| value.clone());
    let validation_mode = read_validation_mode(&value, &invocation_value);
    let mut validation_issues = if validation_mode == ValidationMode::Off {
        Vec::new()
    } else {
        validation::validate_invocation_schema(&invocation_value)
    };
    if validation_mode == ValidationMode::Error && !validation_issues.is_empty() {
        return validation_error_payload(&validation_issues, None);
    }

    let mut invocation = match parse_invocation_value(&value) {
        Ok(invocation) => invocation,
        Err(err) => {
            if !validation_issues.is_empty() {
                return validation_error_payload(&validation_issues, Some(&err.to_string()));
            }
            return error_payload(
                "AC_SCHEMA_INVALID",
                "invalid invocation",
                Some(serde_json::Value::String(err.to_string())),
            );
        }
    };
    eprintln!(
        "DEBUG invocation payload: {}",
        serde_json::to_string(&invocation.payload).unwrap_or_else(|_| "\"<error>\"".to_string())
    );
    // Allow the operation name to steer mode selection if the host provides it.
    if operation.eq_ignore_ascii_case("validate") {
        invocation.mode = InvocationMode::Validate;
    }
    match handle_invocation(invocation) {
        Ok(mut result) => {
            if validation_mode != ValidationMode::Off {
                result.validation_issues.append(&mut validation_issues);
            }
            serde_json::to_string(&result).unwrap_or_else(|err| {
                error_payload(
                    "AC_INTERNAL_ERROR",
                    "serialization error",
                    Some(serde_json::Value::String(err.to_string())),
                )
            })
        }
        Err(err) => {
            if !validation_issues.is_empty() {
                return validation_error_payload(&validation_issues, Some(&err.to_string()));
            }
            error_payload_from_error(&err)
        }
    }
}

pub fn handle_invocation(
    mut invocation: AdaptiveCardInvocation,
) -> Result<AdaptiveCardResult, ComponentError> {
    let state_loaded = state_store::load_state_if_missing(&mut invocation, None)?;
    let state_read_hash = state_loaded.as_ref().and_then(trace::hash_value);
    if let Some(interaction) = invocation.interaction.as_ref()
        && interaction.enabled == Some(false)
    {
        invocation.interaction = None;
    }
    if invocation.interaction.is_some() {
        return handle_interaction(&invocation);
    }

    let rendered = render_card(&invocation)?;
    if invocation.validation_mode == ValidationMode::Error && !rendered.validation_issues.is_empty()
    {
        return Err(ComponentError::CardValidation(rendered.validation_issues));
    }
    let rendered_card = match invocation.mode {
        InvocationMode::Validate => None,
        InvocationMode::Render | InvocationMode::RenderAndValidate => Some(rendered.card),
    };

    let mut telemetry_events = Vec::new();
    if trace::trace_enabled() {
        let state_key = Some(state_store::state_key_for(&invocation, None));
        telemetry_events.push(trace::build_trace_event(
            &invocation,
            &rendered.asset_resolution,
            &rendered.binding_summary,
            None,
            state_key,
            state_read_hash,
            None,
        ));
    }

    Ok(AdaptiveCardResult {
        rendered_card,
        event: None,
        state_updates: Vec::new(),
        session_updates: Vec::new(),
        card_features: rendered.features,
        validation_issues: rendered.validation_issues,
        telemetry_events,
    })
}

#[derive(serde::Deserialize, Default)]
struct InvocationEnvelope {
    #[serde(default)]
    config: Option<AdaptiveCardInvocation>,
    #[serde(default)]
    payload: serde_json::Value,
    #[serde(default)]
    session: serde_json::Value,
    #[serde(default)]
    state: serde_json::Value,
    #[serde(default)]
    interaction: Option<CardInteraction>,
    #[serde(default)]
    mode: Option<InvocationMode>,
    #[serde(default)]
    #[serde(alias = "validationMode")]
    validation_mode: Option<ValidationMode>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    envelope: Option<greentic_types::InvocationEnvelope>,
}

fn parse_invocation_value(
    value: &serde_json::Value,
) -> Result<AdaptiveCardInvocation, ComponentError> {
    if let Some(invocation_value) = validation::locate_invocation_candidate(value) {
        return serde_json::from_value::<AdaptiveCardInvocation>(invocation_value)
            .map_err(ComponentError::Serde);
    }

    if let Some(inner) = value.get("config") {
        if let Ok(invocation) = serde_json::from_value::<AdaptiveCardInvocation>(inner.clone()) {
            return merge_envelope(invocation, value);
        }
        if let Some(card) = inner.get("card")
            && let Ok(invocation) = serde_json::from_value::<AdaptiveCardInvocation>(card.clone())
        {
            return merge_envelope(invocation, value);
        }
    }

    let mut env: InvocationEnvelope = serde_json::from_value(value.clone())?;
    if env.config.is_none()
        && let Ok(invocation) =
            serde_json::from_value::<AdaptiveCardInvocation>(env.payload.clone())
    {
        return Ok(invocation);
    }
    let config = env.config.take().unwrap_or_default();
    Ok(merge_envelope_struct(config, env))
}

fn merge_envelope(
    mut inv: AdaptiveCardInvocation,
    value: &serde_json::Value,
) -> Result<AdaptiveCardInvocation, ComponentError> {
    let env: serde_json::Value = value.clone();
    let payload = env
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let session = env
        .get("session")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let state = env.get("state").cloned().unwrap_or(serde_json::Value::Null);
    if let Some(node_id) = env.get("node_id").and_then(|v| v.as_str()) {
        inv.node_id = Some(node_id.to_string());
    }
    if !payload.is_null() {
        inv.payload = payload;
    }
    if !session.is_null() {
        inv.session = session;
    }
    if !state.is_null() {
        inv.state = state;
    }
    if inv.interaction.is_none()
        && let Some(interaction) = env.get("interaction")
    {
        inv.interaction = serde_json::from_value(interaction.clone()).ok();
    }
    if let Some(mode) = env.get("mode")
        && let Ok(parsed) = serde_json::from_value::<InvocationMode>(mode.clone())
    {
        inv.mode = parsed;
    }
    if let Some(mode_value) = env
        .get("validation_mode")
        .or_else(|| env.get("validationMode"))
        && let Some(parsed) = parse_validation_mode(mode_value)
    {
        inv.validation_mode = parsed;
    }
    if let Some(envelope) = env.get("envelope") {
        inv.envelope = serde_json::from_value(envelope.clone()).ok();
    }
    Ok(inv)
}

fn merge_envelope_struct(
    mut inv: AdaptiveCardInvocation,
    env: InvocationEnvelope,
) -> AdaptiveCardInvocation {
    if inv.card_spec.inline_json.is_none()
        && let Ok(candidate) = serde_json::from_value::<AdaptiveCardInvocation>(env.payload.clone())
    {
        return candidate;
    }
    if env.node_id.is_some() {
        inv.node_id = env.node_id;
    }
    if !env.payload.is_null() {
        inv.payload = env.payload;
    }
    if !env.session.is_null() {
        inv.session = env.session;
    }
    if !env.state.is_null() {
        inv.state = env.state;
    }
    if inv.interaction.is_none() {
        inv.interaction = env.interaction;
    }
    if let Some(mode) = env.mode {
        inv.mode = mode;
    }
    if let Some(mode) = env.validation_mode {
        inv.validation_mode = mode;
    }
    if env.envelope.is_some() {
        inv.envelope = env.envelope;
    }
    inv
}

fn error_payload(code: &str, message: &str, details: Option<serde_json::Value>) -> String {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "code".to_string(),
        serde_json::Value::String(code.to_string()),
    );
    payload.insert(
        "message".to_string(),
        serde_json::Value::String(message.to_string()),
    );
    if let Some(details) = details {
        payload.insert("details".to_string(), details);
    }
    serde_json::json!({ "error": payload }).to_string()
}

fn validation_error_payload(issues: &[ValidationIssue], detail: Option<&str>) -> String {
    let mut message = "invocation schema validation failed".to_string();
    if let Some(detail) = detail {
        message = format!("{message}: {detail}");
    }
    let details = serde_json::json!({ "validation_issues": issues });
    error_payload("AC_SCHEMA_INVALID", &message, Some(details))
}

fn error_payload_from_error(err: &ComponentError) -> String {
    let issue_details = |code: &str, message: String, path: &str| {
        serde_json::json!({
            "validation_issues": [{
                "code": code,
                "message": message,
                "path": path
            }]
        })
    };
    match err {
        ComponentError::InvalidInput(message) => error_payload(
            "AC_SCHEMA_INVALID",
            "invalid input",
            Some(issue_details("AC_SCHEMA_INVALID", message.clone(), "/")),
        ),
        ComponentError::Serde(inner) => error_payload(
            "AC_SCHEMA_INVALID",
            "invalid input",
            Some(issue_details("AC_SCHEMA_INVALID", inner.to_string(), "/")),
        ),
        ComponentError::Io(inner) => error_payload(
            "AC_SCHEMA_INVALID",
            "io error",
            Some(issue_details("AC_SCHEMA_INVALID", inner.to_string(), "/")),
        ),
        ComponentError::AssetNotFound(path) => error_payload(
            "AC_ASSET_NOT_FOUND",
            "asset not found",
            Some(issue_details(
                "AC_ASSET_NOT_FOUND",
                path.clone(),
                "/card_spec",
            )),
        ),
        ComponentError::AssetParse(message) => error_payload(
            "AC_ASSET_PARSE_ERROR",
            "asset parse error",
            Some(issue_details(
                "AC_ASSET_PARSE_ERROR",
                message.clone(),
                "/card_spec",
            )),
        ),
        ComponentError::Asset(message) => error_payload(
            "AC_ASSET_NOT_FOUND",
            "asset error",
            Some(issue_details(
                "AC_ASSET_NOT_FOUND",
                message.clone(),
                "/card_spec",
            )),
        ),
        ComponentError::Binding(message) => error_payload(
            "AC_BINDING_EVAL_ERROR",
            "binding evaluation error",
            Some(issue_details(
                "AC_BINDING_EVAL_ERROR",
                message.clone(),
                "/card_spec/inline_json",
            )),
        ),
        ComponentError::CardValidation(issues) => {
            let details = serde_json::json!({ "validation_issues": issues });
            error_payload(
                "AC_CARD_VALIDATION_FAILED",
                "card validation failed",
                Some(details),
            )
        }
        ComponentError::InteractionInvalid(message) => error_payload(
            "AC_INTERACTION_INVALID",
            "interaction invalid",
            Some(issue_details(
                "AC_INTERACTION_INVALID",
                message.clone(),
                "/interaction",
            )),
        ),
        ComponentError::StateStore(message) => error_payload(
            "AC_SCHEMA_INVALID",
            "state store error",
            Some(issue_details(
                "AC_SCHEMA_INVALID",
                message.clone(),
                "/state",
            )),
        ),
    }
}

fn read_validation_mode(
    value: &serde_json::Value,
    invocation_value: &serde_json::Value,
) -> ValidationMode {
    invocation_value
        .get("validation_mode")
        .or_else(|| invocation_value.get("validationMode"))
        .or_else(|| value.get("validation_mode"))
        .or_else(|| value.get("validationMode"))
        .and_then(parse_validation_mode)
        .unwrap_or_default()
}

fn parse_validation_mode(value: &serde_json::Value) -> Option<ValidationMode> {
    let raw = value.as_str()?.to_ascii_lowercase();
    match raw.as_str() {
        "off" => Some(ValidationMode::Off),
        "warn" => Some(ValidationMode::Warn),
        "error" => Some(ValidationMode::Error),
        _ => None,
    }
}

#[cfg(test)]
mod debug_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_payload_value() {
        let input = json!({
            "card_spec": {
                "inline_json": {
                    "type": "AdaptiveCard",
                    "version": "1.3",
                    "body": [
                        { "type": "TextBlock", "text": "@{payload.title}" }
                    ]
                }
            },
            "payload": {
                "title": "Hello"
            }
        });
        let invocation = parse_invocation_value(&input).expect("should parse");
        println!("payload: {}", invocation.payload);
    }
}
