#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
KEEP_TEMP="${KEEP_TEMP:-0}"
PACK_DIR="${TMP_DIR}/pack"
PACK_ID="ai.greentic.component-adaptive-card-test"
FLOW_ID="ai.greentic.component-adaptive-card.smoke"
NODE_ID="adaptive_card_step"
COMPONENT_ID="ai.greentic.component-adaptive-card"
FLOW_FILE="${PACK_DIR}/flows/adaptive-card.ygtc"
WASM_SRC="${ROOT_DIR}/target/wasm32-wasip2/release/component_adaptive_card.wasm"
NO_STATE_TARGET_DIR="${TMP_DIR}/target_no_state"
NO_STATE_WASM="${NO_STATE_TARGET_DIR}/wasm32-wasip2/release/component_adaptive_card.wasm"

cleanup() {
  if [[ "${KEEP_TEMP}" == "1" ]]; then
    echo "Keeping temp dir: ${TMP_DIR}"
    return
  fi
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

echo "Temp dir: ${TMP_DIR}"

make -C "${ROOT_DIR}" build

cargo build --release --target wasm32-wasip2 --no-default-features --target-dir "${NO_STATE_TARGET_DIR}"
WASM_SRC="${NO_STATE_WASM}"

if [[ ! -f "${WASM_SRC}" ]]; then
  echo "Missing wasm artifact at ${WASM_SRC}" >&2
  exit 1
fi

greentic-pack new --dir "${PACK_DIR}" "${PACK_ID}"
mkdir -p "${PACK_DIR}/components"
cp "${WASM_SRC}" "${PACK_DIR}/components/${COMPONENT_ID}.wasm"
cp "${ROOT_DIR}/component.manifest.json" "${PACK_DIR}/components/component.manifest.json"
sed -i \
  "s|\"component_wasm\": \"target/wasm32-wasip2/release/component_adaptive_card.wasm\"|\"component_wasm\": \"${COMPONENT_ID}.wasm\"|" \
  "${PACK_DIR}/components/component.manifest.json"

greentic-flow new \
  --flow "${FLOW_FILE}" \
  --id "${FLOW_ID}" \
  --type messaging \
  --name "Adaptive Card smoke" \
  --description "Smoke test for component-adaptive-card"

cat > "${TMP_DIR}/payload.json" <<'JSON'
{
  "card_source": "inline",
  "card_spec": {
    "inline_json": {
      "type": "AdaptiveCard",
      "version": "1.6",
      "body": [
        { "type": "TextBlock", "text": "Hello @{payload.user.name}" },
        { "type": "TextBlock", "text": "Input: @{payload.input_name}" },
        { "type": "TextBlock", "text": "${payload.user.tier == \"pro\" ? \"Tier Pro\" : \"Tier Standard\"}" },
        { "type": "TextBlock", "text": "Title: @{params.title||\"Welcome\"}" }
      ],
      "actions": [
        { "type": "Action.Submit", "title": "Continue", "id": "continue" }
      ]
    },
    "template_params": {
      "title": "Onboarding"
    }
  },
  "payload": {
    "user": {
      "name": "Ada",
      "tier": "pro"
    }
    ,
    "input_name": "ExplicitAda"
  },
  "interaction": {
    "interaction_type": "Submit",
    "action_id": "continue",
    "card_instance_id": "card-1",
    "raw_inputs": { "agree": true },
    "metadata": { "route": "next_step", "cardId": "welcome" }
  },
  "mode": "renderAndValidate"
}
JSON

greentic-flow add-step \
  --flow "${FLOW_FILE}" \
  --node-id "${NODE_ID}" \
  --operation card \
  --payload "$(cat "${TMP_DIR}/payload.json")" \
  --local-wasm "${PACK_DIR}/components/${COMPONENT_ID}.wasm" \
  --routing-out

greentic-pack update --in "${PACK_DIR}"
greentic-pack build --in "${PACK_DIR}"

PACK_ARCHIVE="${PACK_DIR}/dist/$(basename "${PACK_DIR}").gtpack"
RUN_OUTPUT="$(greentic-runner-cli --pack "${PACK_ARCHIVE}" --flow "${FLOW_ID}" --input '{}' --json | tail -n 1)"

if command -v rg >/dev/null 2>&1; then
  ARTIFACTS_DIR="$(echo "${RUN_OUTPUT}" | rg -o '"artifacts_dir":"[^"]+"' | head -n1 | sed 's/.*"artifacts_dir":"//;s/"$//')"
else
  ARTIFACTS_DIR="$(echo "${RUN_OUTPUT}" | grep -o '"artifacts_dir":"[^"]\+"' | head -n1 | sed 's/.*"artifacts_dir":"//;s/"$//')"
fi
if [[ -z "${ARTIFACTS_DIR}" ]]; then
  echo "Could not locate artifacts_dir in runner output." >&2
  echo "${RUN_OUTPUT}" >&2
  exit 1
fi

ARTIFACTS_PATH="${ROOT_DIR}/${ARTIFACTS_DIR#./}"
TRANSCRIPT="${ARTIFACTS_PATH}/transcript.jsonl"
if [[ ! -f "${TRANSCRIPT}" ]]; then
  echo "Missing transcript at ${TRANSCRIPT}" >&2
  exit 1
fi

if command -v rg >/dev/null 2>&1; then
  OUTPUT_LINE="$(rg '"phase":"end"' "${TRANSCRIPT}" | tail -n 1)"
else
  OUTPUT_LINE="$(grep -e '"phase":"end"' "${TRANSCRIPT}" | tail -n 1)"
fi
if [[ -z "${OUTPUT_LINE}" ]]; then
  echo "Missing end phase entry in transcript." >&2
  exit 1
fi

if command -v jq >/dev/null 2>&1; then
  RESULT="$(echo "${OUTPUT_LINE}" | jq -c '.. | objects | select(has("renderedCard")) | {renderedCard, event} ' | head -n 1)"
  if [[ -z "${RESULT}" ]]; then
    echo "Did not find rendered_card in transcript." >&2
    echo "${OUTPUT_LINE}" >&2
    exit 1
  fi

  echo "${RESULT}" | jq -e '.renderedCard.body[0].text == "Hello Ada"' >/dev/null || {
    echo "Rendered card did not include expected greeting." >&2
    echo "${RESULT}" >&2
    exit 1
  }
  echo "${RESULT}" | jq -e '.renderedCard.body[1].text == "Input: ExplicitAda"' >/dev/null || {
    echo "Rendered card did not include explicit payload input value." >&2
    echo "${RESULT}" >&2
    exit 1
  }
  echo "${RESULT}" | jq -e '.renderedCard.body[2].text == "Tier Pro"' >/dev/null || {
    echo "Rendered card did not include evaluated expression." >&2
    echo "${RESULT}" >&2
    exit 1
  }
  echo "${RESULT}" | jq -e '.renderedCard.actions[0].type == "Action.Submit"' >/dev/null || {
    echo "Rendered card missing expected action." >&2
    echo "${RESULT}" >&2
    exit 1
  }
  echo "${RESULT}" | jq -e '.event.actionId == "continue"' >/dev/null || {
    echo "Interaction event missing expected action_id." >&2
    echo "${RESULT}" >&2
    exit 1
  }
else
  echo "${OUTPUT_LINE}" | grep -q '"Hello Ada"' || {
    echo "Rendered card did not include expected greeting." >&2
    echo "${OUTPUT_LINE}" >&2
    exit 1
  }
  echo "${OUTPUT_LINE}" | grep -q '"Input: ExplicitAda"' || {
    echo "Rendered card did not include explicit payload input value." >&2
    echo "${OUTPUT_LINE}" >&2
    exit 1
  }
  echo "${OUTPUT_LINE}" | grep -q '"Action.Submit"' || {
    echo "Rendered card missing expected action." >&2
    echo "${OUTPUT_LINE}" >&2
    exit 1
  }
  echo "${OUTPUT_LINE}" | grep -q '"actionId":"continue"' || {
    echo "Interaction event missing expected action_id." >&2
    echo "${OUTPUT_LINE}" >&2
    exit 1
  }
fi

echo "Smoke test passed."
