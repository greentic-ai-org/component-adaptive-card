# Adaptive Card Component

The `component-adaptive-card` renders Adaptive Card v1.6 payloads and normalizes user interactions for Greentic flows. It supports inline cards, asset-backed cards, catalog lookup, binding/expression expansion, validation, and interaction-derived state/session updates.

## Quick start with greentic-dev

Create a flow file and add a component step using `greentic-dev add-step` (note the component name is `adaptive-card`):

```bash
greentic-dev add-step \
  --flow flows/adaptive-card.ygtc \
  --id render_card \
  --component adaptive-card \
  --operation card \
  --input examples/adaptive-card.inline.json
```

Example input (`examples/adaptive-card.inline.json`):

```json
{
  "card_source": "inline",
        "card_spec": {
          "inline_json": {
            "type": "AdaptiveCard",
            "version": "1.6",
            "body": [
              { "type": "TextBlock", "text": "Hello {{payload.user.name}}" }
            ]
          }
        }
}
```

Validate the flow:

```bash
greentic-dev flow validate -f flows/adaptive-card.ygtc --json
```

## Advanced example (assets, bindings, interactions)

Render a catalog card, inject parameters, and validate output:

```bash
greentic-dev add-step \
  --flow flows/adaptive-card-advanced.ygtc \
  --id render_card \
  --component adaptive-card \
  --operation card \
  --input examples/adaptive-card.catalog.json
```

Example input (`examples/adaptive-card.catalog.json`):

```json
{
  "card_source": "catalog",
  "card_spec": {
    "catalog_name": "onboarding",
    "template_params": {
      "title": "Welcome",
      "show_help": true
    },
    "asset_registry": {
      "onboarding": "assets/cards/onboarding.json"
    }
  },
  "payload": {
    "user": { "name": "Ada", "tier": "pro" }
  },
  "mode": "renderAndValidate"
}
```

If you need to process an interaction from the host, include the `interaction` object in the input payload:

```json
{
  "interaction": {
    "interaction_type": "Submit",
    "action_id": "start",
    "card_instance_id": "card-1",
    "raw_inputs": { "agree": true },
    "metadata": { "route": "next_step", "cardId": "onboarding" }
  }
}
```

## Input reference

The component exposes one operation: `card`.

- `card_source`: `inline` (default), `asset`, or `catalog`.
- `card_spec`:
  - `inline_json`: Inline Adaptive Card JSON (object or array).
  - `asset_path`: Direct path to a JSON file on disk.
  - `catalog_name`: Logical name resolved to `<base>/<name>.json`.
  - `template_params`: JSON object/array exposed to bindings as `params.*` or `template.*`.
  - `asset_registry`: Optional map of logical names to paths (overrides env registry).
- `node_id`: Optional node id used to expose `node` and `node_payload` in Handlebars.
- `payload`, `session`, `state`: JSON contexts used by binding/expression resolution.
- `interaction`: Optional interaction payload (see Interaction handling).
- `mode`: `render`, `validate`, or `renderAndValidate` (default).
- `envelope`: Optional `greentic_types::InvocationEnvelope` metadata.

Defaults:
- `card_source` defaults to `inline`.
- `card_spec` defaults to `{ "inline_json": {} }`.
- `mode` defaults to `renderAndValidate`.
- `payload`, `session`, and `state` default to `{}` when omitted.

## Card sources and asset resolution

- **inline:** Uses `card_spec.inline_json`.
- **asset:** Uses `card_spec.asset_path`, or resolves via registry/base path.
- **catalog:** Resolves `catalog_name` to `<base>/<name>.json` after registry lookup.

Registry and base path:
- `card_spec.asset_registry` takes precedence for both asset and catalog lookups.
- `ADAPTIVE_CARD_ASSET_REGISTRY` can point to a JSON map on disk.
- `ADAPTIVE_CARD_CATALOG_FILE` can point to a JSON map for catalog names.
- `ADAPTIVE_CARD_ASSET_BASE` controls the base folder (default `assets`).
- In wasm32 builds, on-disk loading is disabled; use the host asset resolver or inline JSON.

## Bindings, expressions, and Handlebars

The component traverses the card JSON and applies Handlebars first, then replaces placeholders:

- `{{...}}`: Handlebars templates with access to `payload`, `state`, and prior node outputs (see below).
- `@{path}`: Path lookup with typed replacement for whole-string values.
- `@{path||default}`: Provides a default when the value is missing or null.
- `${expr}`: Expression evaluation (whole-string only), supports dotted paths, `==`, and ternary `cond ? a : b`.
- Embedded placeholders (inside larger strings) are replaced as strings.

Resolution order for bare paths: `payload`, then `session`, then `state`, then `params`/`template`.

### Handlebars context

Handlebars receives this context:

- `payload`: the node input payload (explicit access via `{{payload.foo}}`).
- `state`: the runner execution snapshot (explicit access via `{{state.input.foo}}` or `{{state.nodes.prev.payload.answer}}`).
- Implicit lookups (`{{name}}`) resolve from `state.input.name` when present.
- `payload` remains explicit and is not shadowed by implicit lookups.
- If `node_id` is provided:
  - `node`: the full `state.nodes.<node_id>` object.
  - `node_payload`: the `state.nodes.<node_id>.payload` shortcut.

## Interaction handling

When `interaction` is present, the component:

- Emits an `event` describing the action (`Submit`, `Execute`, `OpenUrl`, `ShowCard`, `ToggleVisibility`).
- Adds `state_updates`:
  - `Submit`/`Execute`: merges into `form_data`.
  - `ShowCard`: sets `ui.active_show_card.<card_instance_id>`.
  - `ToggleVisibility`: sets `ui.visibility.<action_id>`.
- Adds `session_updates` when `interaction.metadata.route` is set.

## Output

The result includes:

- `rendered_card`: Canonical Adaptive Card JSON (omitted when `mode=validate`).
- `event`: Optional interaction event metadata.
- `state_updates` and `session_updates`: Declarative ops for the host.
- `card_features`: Feature summary (elements, actions, media, auth, etc).
- `validation_issues`: Structural validation findings.

## Validation

Validation checks core Adaptive Card invariants (root type, version, element/action shape, input rules). Use `mode=validate` to skip rendering and return validation issues only.

## GHCR images

This component publishes OCI artifacts to GHCR as:

```
ghcr.io/<org>/components/component-adaptive-card:<version>
```

`<version>` comes from `Cargo.toml`/`component.manifest.json` and is also tagged as `latest`.

## Development

Developer notes live in `dev.md`.
