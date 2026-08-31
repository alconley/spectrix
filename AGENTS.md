# Spectrix contributor checks

## Required verification

Before handing off a Rust change, run these commands from the repository root and resolve every failure:

```powershell
cargo fmt --check
cargo clippy -- -D warnings
```

Run the smallest relevant test suite as well. For changes that affect shared crates, persistence, fitting, or serialization, run the relevant workspace/package tests rather than only the UI binary check.

Do not suppress a formatter, Clippy, or compiler warning merely to pass a check unless there is a documented, narrowly scoped reason.

## Rust safety

Do not introduce Rust `unsafe` code. This includes `unsafe` blocks, functions, traits, implementations, and attributes that permit unsafe behavior. Prefer safe Rust and supported library APIs; if an existing unsafe dependency or implementation needs to be changed, preserve its safety boundary and ask before expanding it.

## UI guidance and hover help

Interactive UI elements must explain themselves when a user could reasonably need context to use them correctly. Add concise hover text/tooltips for controls that have non-obvious behavior, including drag targets, editable plot elements, buttons, toggles, inputs, icons, keyboard shortcuts, bounds, units, defaults, side effects, and feature interactions.

- State what the element does and, where relevant, how to use it (for example, which drag direction changes which value).
- Include essential constraints or interactions a user needs to make a safe or informed choice, such as shared-vs-independent behavior, destructive effects, required prerequisites, or persistence behavior.
- Do not add redundant hover text for universally obvious controls when their label already fully explains the action.
- Keep tooltip wording short, plain-language, and aligned with the currently implemented behavior.

## Persistence and backward compatibility

Spectrix is persistence-first. Saved application state, fit data, preferences, markers, and other user-authored data must survive feature additions and version upgrades.

- Adding a field must preserve existing saved values. Use a meaningful default only when an older file has no value for the new field; never replace or reset a value that was loaded successfully.
- For serde-backed structs that evolve, use backward-compatible defaults (for example, `#[serde(default)]` on the struct and safe `Default` values for newly added fields).
- Treat deserialization/migration code as an upgrade path: retain all recognized existing data, migrate legacy representations explicitly, and avoid destructive fallbacks.
- New features should begin with their default configuration for existing users, while all pre-existing settings and data remain unchanged.
- Add or update serialization compatibility tests whenever a persisted schema changes, including loading representative older data when applicable.
- Do not clear, recreate, or overwrite persisted state as a side effect of opening data, applying defaults, or adding a UI feature.

When a persistence behavior is ambiguous, prefer preserving the prior state and ask before making a destructive migration choice.
