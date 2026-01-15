use serde_json::{Map, Value};

use crate::error::ComponentError;
use crate::model::{
    AdaptiveActionEvent, AdaptiveActionType, AdaptiveCardInvocation, AdaptiveCardResult,
    CardInteractionType, SessionUpdateOp, StateUpdateOp,
};
use crate::render::render_card;
use crate::state_store;

pub fn handle_interaction(
    inv: &AdaptiveCardInvocation,
) -> Result<AdaptiveCardResult, ComponentError> {
    let interaction = inv
        .interaction
        .clone()
        .ok_or_else(|| ComponentError::InvalidInput("interaction is required".into()))?;

    let mut invocation = inv.clone();
    state_store::load_state_if_missing(&mut invocation, Some(&interaction))?;
    let resolved = render_card(&invocation)?;
    let normalized_inputs = normalize_inputs(&interaction.raw_inputs);
    let mut state_updates = Vec::new();
    let mut session_updates = Vec::new();

    if let Some(route) = interaction
        .metadata
        .get("route")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
    {
        session_updates.push(SessionUpdateOp::SetRoute { route });
    }

    let action_type = match interaction.interaction_type {
        CardInteractionType::Submit => {
            state_updates.push(StateUpdateOp::Merge {
                path: "form_data".into(),
                value: normalized_inputs.clone(),
            });
            AdaptiveActionType::Submit
        }
        CardInteractionType::Execute => {
            state_updates.push(StateUpdateOp::Merge {
                path: "form_data".into(),
                value: normalized_inputs.clone(),
            });
            AdaptiveActionType::Execute
        }
        CardInteractionType::OpenUrl => AdaptiveActionType::OpenUrl,
        CardInteractionType::ShowCard => {
            let subcard_id = interaction
                .metadata
                .get("subcardId")
                .and_then(|v| v.as_str())
                .unwrap_or(&interaction.action_id)
                .to_string();
            state_updates.push(StateUpdateOp::Set {
                path: format!("ui.active_show_card.{}", interaction.card_instance_id),
                value: Value::String(subcard_id.clone()),
            });
            AdaptiveActionType::ShowCard
        }
        CardInteractionType::ToggleVisibility => {
            let visible = interaction
                .metadata
                .get("visible")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            state_updates.push(StateUpdateOp::Set {
                path: format!("ui.visibility.{}", interaction.action_id),
                value: Value::Bool(visible),
            });
            AdaptiveActionType::ToggleVisibility
        }
    };

    let event = AdaptiveActionEvent {
        action_type,
        action_id: interaction.action_id.clone(),
        verb: interaction.verb.clone(),
        route: interaction
            .metadata
            .get("route")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        inputs: normalized_inputs.clone(),
        card_id: interaction
            .metadata
            .get("cardId")
            .and_then(|v| v.as_str())
            .unwrap_or(&interaction.card_instance_id)
            .to_string(),
        card_instance_id: interaction.card_instance_id.clone(),
        subcard_id: interaction
            .metadata
            .get("subcardId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        metadata: interaction.metadata.clone(),
    };

    let mut persisted_state = if invocation.state.is_null() {
        Value::Object(Map::new())
    } else {
        invocation.state.clone()
    };
    state_store::apply_updates(&mut persisted_state, &state_updates);
    state_store::persist_state(&invocation, Some(&interaction), &persisted_state)?;

    Ok(AdaptiveCardResult {
        rendered_card: Some(resolved.card),
        event: Some(event),
        state_updates,
        session_updates,
        card_features: resolved.features,
        validation_issues: resolved.validation_issues,
        telemetry_events: Vec::new(),
    })
}

fn normalize_inputs(raw: &Value) -> Value {
    match raw {
        Value::Object(_) => raw.clone(),
        Value::Null => Value::Object(Map::new()),
        Value::String(s) => serde_json::from_str(s).unwrap_or_else(|_| {
            let mut map = Map::new();
            map.insert("value".into(), Value::String(s.clone()));
            Value::Object(map)
        }),
        other => {
            let mut map = Map::new();
            map.insert("value".into(), other.clone());
            Value::Object(map)
        }
    }
}
