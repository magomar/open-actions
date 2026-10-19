---
type: validation_receipt
schema_version: "1.0"
spec: "specs/003_hue_control.md"
epic: "open-actions-m7s"
candidate_commit: "0741257683ff336c9859b1f7ce90300e2fa50716"
verifier: "local-user"
evaluated_at: "2026-10-19T15:54:33Z"
command: "cargo test --manifest-path actions/hue-control/Cargo.toml"
exit_code: 0
duration_ms: 63
status: passed
---

# 🧾 Validation Receipt: Spec 003

- **Candidate Commit**: `0741257683ff336c9859b1f7ce90300e2fa50716`
- **Spec**: `specs/003_hue_control.md`
- **Command**: `cargo test --manifest-path actions/hue-control/Cargo.toml`
- **Result**: `passed` (exit code: 0)

### Verified Criteria
This receipt records only the explicit command result. It does not assert coverage of every Pseudo-Gherkin scenario.

### Execution Log Summary
```
stdout:

running 32 tests
test bridge::tests::accepts_success_payloads ... ok
test bridge::tests::accepts_non_array_payloads ... ok
test bridge::tests::deserializes_discovery_results ... ok
test bridge::tests::extracts_the_username_from_a_pairing_response ... ok
test bridge::tests::maps_entries_in_numeric_id_order ... ok
test bridge::tests::pairing_response_without_success_has_no_username ... ok
test bridge::tests::recognises_the_pending_link_button_error ... ok
test bridge::tests::rejects_in_band_api_errors ... ok
test bridge::tests::config_id_requires_an_ip ... ok
test bridge::tests::pairing_requires_an_ip ... ok
test icon::tests::temperature_colors_follow_the_warm_cool_axis ... ok
test icon::tests::power_states_render_differently ... ok
test settings::tests::accepts_hex_with_or_without_the_hash ... ok
test icon::tests::brightness_clamps_above_one_hundred ... ok
test settings::tests::converts_percent_to_bridge_brightness ... ok
test settings::tests::builds_bridge_paths_per_target_kind ... ok
test settings::tests::converts_red_to_the_bridge_xy_point ... ok
test bridge::tests::requires_both_ip_and_username ... ok
test settings::tests::cycle_advances_and_wraps ... ok
test settings::tests::cycle_handles_an_empty_palette ... ok
test settings::tests::cycle_helpers_filter_and_fallback_properly ... ok
test settings::tests::decodes_group_and_light_targets ... ok
test settings::tests::defaults_are_usable_without_inspector_configuration ... ok
test settings::tests::deserializes_settings_from_a_partial_inspector_payload ... ok
test settings::tests::invalid_palette_entries_fall_back_to_the_default_triad ... ok
test settings::tests::malformed_colors_fall_back_to_a_default ... ok
test settings::tests::normalizes_hex_colors ... ok
test settings::tests::picks_the_only_configured_bridge_when_unnamed ... ok
test settings::tests::rejects_malformed_colors ... ok
test settings::tests::rejects_malformed_targets ... ok
test icon::tests::every_icon_renders_as_an_svg_data_image ... ok
test settings::tests::warmth_spans_the_bridge_mired_range ... ok

test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


stderr:
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/main.rs (actions/hue-control/target/debug/deps/hue_control-79da5ffc5e5dfa55)

```
