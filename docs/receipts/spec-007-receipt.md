---
type: validation_receipt
schema_version: "1.0"
spec: "specs/007_gpu_screen_recorder.md"
epic: "open-actions-sp007-iy2"
candidate_commit: "6aa28052ed70406bc9f523e54beb557c321dbd12"
verifier: "local-user"
evaluated_at: "2026-10-25T13:49:32Z"
command: "cargo test --manifest-path actions/gpu-screen-recorder/Cargo.toml"
exit_code: 0
duration_ms: 561
status: passed
---

# 🧾 Validation Receipt: Spec 007

- **Candidate Commit**: `6aa28052ed70406bc9f523e54beb557c321dbd12`
- **Spec**: `specs/007_gpu_screen_recorder.md`
- **Command**: `cargo test --manifest-path actions/gpu-screen-recorder/Cargo.toml`
- **Result**: `passed` (exit code: 0)

### Verified Criteria
This receipt records only the explicit command result. It does not assert coverage of every Pseudo-Gherkin scenario.

### Execution Log Summary
```
stdout:

running 17 tests
test gsr::tests::classifies_monitor_and_screen_targets_as_screen_mode ... ok
test gsr::tests::constructs_status_command ... ok
test gsr::tests::classifies_wayland_portal_window_token_and_x11_window_as_window_mode ... ok
test gsr::tests::formats_elapsed_duration_as_mm_ss_and_h_mm_ss ... ok
test gsr::tests::ignores_replay_buffer_streaming_screenshots_and_info_commands ... ok
test gsr::tests::parses_nul_separated_cmdline_bytes ... ok
test gsr::tests::preserves_configured_custom_path ... ok
test gsr::tests::detects_active_recording_from_mock_proc_directory ... ok
test gsr::tests::resolves_empty_path_to_default_binary ... ok
test gsr::tests::parses_ui_config_and_builds_recorder_command ... ok
test gsr::tests::resolves_daemon_binary_path_and_launch_command ... ok
test tests::elapsed_for_action_matches_only_matching_capture_mode ... ok
test icon::tests::recording_svgs_include_formatted_time_and_rec_badge ... ok
test tests::settings_deserialize_with_default_empty_path ... ok
test icon::tests::renders_all_states_as_svg_data_uris ... ok
test gsr::tests::missing_or_failing_binary_returns_error_and_offline_status ... ok
test gsr::tests::auto_launches_daemon_and_starts_and_stops_recorder_process ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s


stderr:
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/main.rs (actions/gpu-screen-recorder/target/debug/deps/open_actions_gsr-554bd776fe5033ed)

```
