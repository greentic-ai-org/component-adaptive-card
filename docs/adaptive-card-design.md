# Adaptive Card Component Design

This component renders Adaptive Card v1.6 payloads and handles user interactions in a channel-agnostic way. It always emits canonical Adaptive Card JSON plus a small feature summary; channel-specific downsampling is left to `greentic-messaging`.

## Invocation Envelope
- **Invocation:** `AdaptiveCardInvocation` carrying the card source/spec, flow payload, session, state, optional interaction, and desired mode (`Render`, `Validate`, `RenderAndValidate`).
- **Card source:** inline JSON, an asset path, or a catalog name (resolved under `assets/`).
- **Context:** `payload`, `session`, `state`, and optional `template_params` are available for placeholder binding (`@{path}` or `${path}`); whole-string placeholders are replaced with typed values and can specify `||` defaults.
- **Envelope:** Optional `InvocationEnvelope` from `greentic-types` can accompany the invocation for host metadata.
- **Asset resolution:** resolution order is inline JSON (when provided), inline/env registry maps, pack assets under `ADAPTIVE_CARD_ASSET_BASE` (default `assets`), and an optional host resolver implementing `AssetResolver`. Catalog names map to `<base>/<name>.json` after registry lookups.

## Result Structure
- **AdaptiveCardResult:** rendered card (optional for validation-only), optional `AdaptiveActionEvent`, state and session update ops, feature summary, validation issues, and optional telemetry events.
- **Routing:** Actions emit an event with action metadata, inputs, route/verb when available, and card identifiers.

## State & Session Update Model
- **StateUpdateOp:** declarative `Set`, `Merge`, or `Delete` with a dotted path (e.g., `form_data`, `ui.visibility.section`).
- **SessionUpdateOp:** route/attribute updates plus simple card stack push/pop hooks for navigation flows.
- Updates are instructions only; the runner applies them to persistent storage.

## Responsibilities
- **In scope:** card resolution (inline/asset/catalog), placeholder binding from context (typed replacement with `||` defaults for whole-string placeholders), minimal expression evaluation (dotted path lookups, interpolation, equality, ternary) via a pluggable engine, structural validation (root type, version present, input ids/uniqueness, action requirements, basic element shape checks), feature analysis, interaction normalization, and declarative updates/events.
- **Out of scope:** channel rendering/downsampling, network calls, or state/session persistence. The host applies updates and performs delivery.
