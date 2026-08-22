# Conformance map: TS -> Rust

The e2e conformance suite is the behavioral contract of the frozen CLI
surface. It was ported 1:1 from `src/**/*.conformance.test.ts` (TS, retired)
to `rust/tests/*_conformance.rs` (std-only integration tests, run against the
built binary via `env!("CARGO_BIN_EXE_agnosgram")`, or `AGNOSGRAM_BIN` as a
runtime override).

Baseline: the full TS conformance suite (104 cases) was run once against the
Rust binary before any port work started, to confirm 104/104 green - see
`impl-report-agnosgram.md` for the run. Every case below is ported; none were
dropped.

| TS file | TS case | Rust test fn |
|---|---|---|
| `src/cli.conformance.test.ts` | `--version` prints a bare semver-looking string | `cli_conformance::version_prints_a_bare_semver_looking_string` |
| `src/cli.conformance.test.ts` | `--help` prints usage | `cli_conformance::help_prints_usage` |
| `src/cli.conformance.test.ts` | an unknown command exits non-zero without a raw stack trace | `cli_conformance::an_unknown_command_exits_non_zero_without_a_raw_stack_trace` |
| `src/commands/bootstrap.conformance.test.ts` | bootstrap emits a prompt targeting context files and detected stack | `bootstrap_conformance::bootstrap_emits_a_prompt_targeting_context_files_and_detected_stack` |
| `src/commands/bootstrap.conformance.test.ts` | FRI-001: an unknown flag gives a clean UserError, not a raw parseArgs crash | `bootstrap_conformance::fri_001_an_unknown_flag_gives_a_clean_usererror_not_a_raw_parseargs_crash` |
| `src/commands/init.conformance.test.ts` | init scaffolds the full store | `init_conformance::init_scaffolds_the_full_store` |
| `src/commands/init.conformance.test.ts` | init refuses to overwrite without --force | `init_conformance::init_refuses_to_overwrite_without_force` |
| `src/commands/init.conformance.test.ts` | init auto-adapts detected agents | `init_conformance::init_auto_adapts_detected_agents` |
| `src/commands/init.conformance.test.ts` | init --adapt none writes no adapters | `init_conformance::init_adapt_none_writes_no_adapters` |
| `src/commands/init.conformance.test.ts` | init does not scaffold meta/ - opt-in via `feedback` | `init_conformance::init_does_not_scaffold_meta_it_is_opt_in_via_feedback_on_first_use` |
| `src/commands/init.conformance.test.ts` | init records SDD detection into config-driven hints | `init_conformance::init_records_sdd_detection_into_config_driven_hints` |
| `src/commands/init.conformance.test.ts` | FRI-001: init --adapt -none gives the existing validation error | `init_conformance::fri_001_init_adapt_none_gives_the_existing_validation_error_not_a_raw_parseargs_crash` |
| `src/commands/log.conformance.test.ts` | log appends an entry to the current month's journal | `log_conformance::log_appends_an_entry_to_the_current_months_journal` |
| `src/commands/log.conformance.test.ts` | log with no slots throws a helpful error | `log_conformance::log_with_no_slots_throws_a_helpful_error` |
| `src/commands/log.conformance.test.ts` | FRI-001: log --learned "--x" stores the literal value instead of crashing | `log_conformance::fri_001_log_learned_dash_x_stores_the_literal_value_instead_of_crashing` |
| `src/commands/doctor.conformance.test.ts` | doctor on a healthy store exits 0 and reports no issues | `doctor_conformance::doctor_on_a_healthy_store_exits_0_and_reports_no_issues` |
| `src/commands/doctor.conformance.test.ts` | doctor --json prints the report shape | `doctor_conformance::doctor_json_prints_the_report_shape` |
| `src/commands/doctor.conformance.test.ts` | warnings alone keep exit 0 without --strict | `doctor_conformance::warnings_alone_keep_exit_0_without_strict` |
| `src/commands/doctor.conformance.test.ts` | --strict turns warnings into a failing exit code | `doctor_conformance::strict_turns_warnings_into_a_failing_exit_code` |
| `src/commands/doctor.conformance.test.ts` | doctor without a store gives a clean UserError | `doctor_conformance::doctor_without_a_store_gives_a_clean_usererror` |
| `src/commands/doctor.conformance.test.ts` | FRI-001: doctor --format -json gives the existing validation error | `doctor_conformance::fri_001_doctor_format_dash_json_gives_the_existing_validation_error_not_a_raw_parseargs_crash` |
| `src/commands/adapt.conformance.test.ts` | FRI-001: a dash-leading positional gives a clean UserError | `adapt_conformance::fri_001_a_dash_leading_positional_gives_a_clean_usererror_not_a_raw_parseargs_crash` |
| `src/commands/adapt.conformance.test.ts` | FRI-003: CLAUDE.md symlinked to AGENTS.md reports one honest write, not two | `adapt_conformance::fri_003_claude_md_symlinked_to_agents_md_reports_one_honest_write_not_two` |
| `src/commands/adapt.conformance.test.ts` | FRI-003: the reverse symlink direction is also honest | `adapt_conformance::fri_003_the_reverse_symlink_direction_is_also_honest` |
| `src/commands/adapt.conformance.test.ts` | FRI-003: --json reports the write once, with the symlinked adapter as an alias | `adapt_conformance::fri_003_json_reports_the_write_once_with_the_symlinked_adapter_as_an_alias` |
| `src/commands/adapt.conformance.test.ts` | non-symlinked CLAUDE.md and AGENTS.md are still reported independently | `adapt_conformance::non_symlinked_claude_md_and_agents_md_are_still_reported_independently` |
| `src/commands/distill.conformance.test.ts` | distill emits a compaction prompt naming the schema and rules | `distill_conformance::distill_emits_a_compaction_prompt_naming_the_schema_and_rules` |
| `src/commands/distill.conformance.test.ts` | distill --validate passes a well-formed file | `distill_conformance::distill_validate_passes_a_well_formed_file` |
| `src/commands/distill.conformance.test.ts` | distill --validate fails a schema-broken file with a non-zero exit | `distill_conformance::distill_validate_fails_a_schema_broken_file_with_a_non_zero_exit` |
| `src/commands/distill.conformance.test.ts` | distill --archive moves a journal month into archive/ | `distill_conformance::distill_archive_moves_a_journal_month_into_archive` |
| `src/commands/distill.conformance.test.ts` | distill --archive rejects a bad month argument | `distill_conformance::distill_archive_rejects_a_bad_month_argument` |
| `src/commands/distill.conformance.test.ts` | FRI-001: distill --archive -2026-08 gives the existing validation error | `distill_conformance::fri_001_distill_archive_dash_2026_08_gives_the_existing_validation_error_not_a_raw_parseargs_crash` |
| `src/commands/distill.conformance.test.ts` | distill --validate fails frontmatter holding a block scalar | `distill_conformance::distill_validate_fails_frontmatter_holding_a_block_scalar` |
| `src/commands/feedback.conformance.test.ts` | feedback creates meta/friction.md on first use, not before | `feedback_conformance::feedback_creates_meta_friction_md_on_first_use_not_before` |
| `src/commands/feedback.conformance.test.ts` | feedback allocates sequential FRI- ids | `feedback_conformance::feedback_allocates_sequential_fri_ids` |
| `src/commands/feedback.conformance.test.ts` | feedback defaults scope to cli and confidence to medium | `feedback_conformance::feedback_defaults_scope_to_cli_and_confidence_to_medium` |
| `src/commands/feedback.conformance.test.ts` | feedback --scope and --confidence override the defaults | `feedback_conformance::feedback_scope_and_confidence_override_the_defaults` |
| `src/commands/feedback.conformance.test.ts` | feedback rejects an unknown --confidence | `feedback_conformance::feedback_rejects_an_unknown_confidence` |
| `src/commands/feedback.conformance.test.ts` | feedback rejects a --scope tag that would break the YAML flow sequence (bracket) | `feedback_conformance::feedback_rejects_a_scope_tag_that_would_break_the_yaml_flow_sequence_bracket` |
| `src/commands/feedback.conformance.test.ts` | feedback rejects a --scope tag containing a colon-space | `feedback_conformance::feedback_rejects_a_scope_tag_containing_a_colon_space` |
| `src/commands/feedback.conformance.test.ts` | feedback with no text throws a usage error | `feedback_conformance::feedback_with_no_text_throws_a_usage_error` |
| `src/commands/feedback.conformance.test.ts` | feedback --stdin reads the entry text from piped input | `feedback_conformance::feedback_stdin_reads_the_entry_text_from_piped_input` |
| `src/commands/feedback.conformance.test.ts` | feedback --json prints a structured envelope and no gh command by default | `feedback_conformance::feedback_json_prints_a_structured_envelope_and_no_gh_command_by_default` |
| `src/commands/feedback.conformance.test.ts` | feedback --share prints a ready-to-run gh issue create command | `feedback_conformance::feedback_share_prints_a_ready_to_run_gh_issue_create_command_targeting_the_real_repo_but_never_runs_it` |
| `src/commands/feedback.conformance.test.ts` | feedback --share --json includes the command as a string field | `feedback_conformance::feedback_share_json_includes_the_command_targeting_the_real_repo_as_a_string_field` |
| `src/commands/feedback.conformance.test.ts` | feedback writes only under .agnosgram/meta/, touching no other file | `feedback_conformance::feedback_writes_only_under_agnosgram_meta_touching_no_other_file` |
| `src/commands/feedback.conformance.test.ts` | FRI-001: feedback --scope -docs accepts a dash-leading but otherwise valid tag | `feedback_conformance::fri_001_feedback_scope_dash_docs_accepts_a_dash_leading_but_otherwise_valid_tag_not_a_crash` |
| `src/commands/advise.conformance.test.ts` | advise emits a prompt naming the plan, the digest, and the precedence rule verbatim | `advise_conformance::advise_emits_a_prompt_naming_the_plan_the_digest_and_the_precedence_rule_verbatim` |
| `src/commands/advise.conformance.test.ts` | advise --out changes the report path referenced in the prompt | `advise_conformance::advise_out_changes_the_report_path_referenced_in_the_prompt` |
| `src/commands/advise.conformance.test.ts` | FRI-001: advise --out -custom.json accepts a dash-leading path | `advise_conformance::fri_001_advise_out_dash_custom_json_accepts_a_dash_leading_path_not_a_crash` |
| `src/commands/advise.conformance.test.ts` | advise --validate passes a well-formed, provenance-correct report | `advise_conformance::advise_validate_passes_a_well_formed_provenance_correct_report` |
| `src/commands/advise.conformance.test.ts` | advise --validate fails on a confidence provenance mismatch | `advise_conformance::advise_validate_fails_on_a_confidence_provenance_mismatch` |
| `src/commands/advise.conformance.test.ts` | advise --validate fails when a cited record_id does not exist | `advise_conformance::advise_validate_fails_when_a_cited_record_id_does_not_exist` |
| `src/commands/advise.conformance.test.ts` | advise --validate warns (does not error) on an excerpt substring mismatch | `advise_conformance::advise_validate_warns_does_not_error_on_an_excerpt_substring_mismatch` |
| `src/commands/advise.conformance.test.ts` | advise --validate: exit 0 without --strict, exit 1 with --strict, when clear is false but valid | `advise_conformance::advise_validate_exit_0_without_strict_exit_1_with_strict_when_clear_is_false_but_valid` |
| `src/commands/advise.conformance.test.ts` | advise --validate --json returns {file, ok, errors, warnings, issues, report} | `advise_conformance::advise_validate_json_returns_file_ok_errors_warnings_issues_report` |
| `src/commands/advise.conformance.test.ts` | advise --validate rejects malformed JSON | `advise_conformance::advise_validate_rejects_malformed_json` |
| `src/commands/advise.conformance.test.ts` | advise --validate on a missing file throws a usage error | `advise_conformance::advise_validate_on_a_missing_file_throws_a_usage_error` |
| `src/commands/advise.conformance.test.ts` | advise with no plan path throws a usage error | `advise_conformance::advise_with_no_plan_path_throws_a_usage_error` |
| `src/commands/advise.conformance.test.ts` | advise --validate errors (even without --strict) when clear:true coexists with a blocker | `advise_conformance::advise_validate_errors_even_without_strict_when_clear_true_coexists_with_a_blocker` |
| `src/commands/advise.conformance.test.ts` | advise --validate --strict derives exit mechanically: blocker + clear:true still exits 1 | `advise_conformance::advise_validate_strict_derives_exit_mechanically_blocker_plus_clear_true_still_exits_1` |
| `src/commands/advise.conformance.test.ts` | advise --validate fails when checked_ids contains a non-string entry | `advise_conformance::advise_validate_fails_when_checked_ids_contains_a_non_string_entry` |
| `src/commands/advise.conformance.test.ts` | advise --validate falls back to a root-relative plan path from a subdirectory | `advise_conformance::advise_validate_falls_back_to_a_root_relative_plan_path_from_a_subdirectory` |
| `src/commands/advise.conformance.test.ts` | advise --validate resolves an absolute plan path | `advise_conformance::advise_validate_resolves_an_absolute_plan_path` |
| `src/commands/advise.conformance.test.ts` | advise's digest never includes meta/friction.md content | `advise_conformance::advises_digest_never_includes_meta_friction_md_content` |
| `src/commands/advise.conformance.test.ts` | advise digest table escapes pipes in body excerpts and only appends ... when truncated | `advise_conformance::advise_digest_table_escapes_pipes_in_body_excerpts_and_only_appends_ellipsis_when_truncated` |
| `src/commands/advise.conformance.test.ts` | advise digest table appends ... only when the body actually exceeds 80 chars | `advise_conformance::advise_digest_table_appends_ellipsis_only_when_the_body_actually_exceeds_80_chars` |
| `src/commands/pack.conformance.test.ts` | pack includes status.md verbatim and lessons, but not decisions, when unscoped | `pack_conformance::pack_includes_status_md_verbatim_and_lessons_but_not_decisions_when_unscoped` |
| `src/commands/pack.conformance.test.ts` | pack --scope includes matching decisions and filters lessons by scope | `pack_conformance::pack_scope_includes_matching_decisions_and_filters_lessons_by_scope` |
| `src/commands/pack.conformance.test.ts` | pack --budget greedily drops whole records and lists them in an Omitted section | `pack_conformance::pack_budget_greedily_drops_whole_records_and_lists_them_in_an_omitted_section` |
| `src/commands/pack.conformance.test.ts` | pack --json returns the pinned shape | `pack_conformance::pack_json_returns_the_pinned_shape` |
| `src/commands/pack.conformance.test.ts` | pack --budget overrides config pack_budget, which overrides the 2000 default | `pack_conformance::pack_budget_overrides_config_pack_budget_which_overrides_the_2000_default` |
| `src/commands/pack.conformance.test.ts` | pack default budget is 2000 when nothing overrides it | `pack_conformance::pack_default_budget_is_2000_when_nothing_overrides_it` |
| `src/commands/pack.conformance.test.ts` | pack exits non-zero with guidance when state/status.md is missing | `pack_conformance::pack_exits_non_zero_with_guidance_when_state_status_md_is_missing` |
| `src/commands/pack.conformance.test.ts` | pack tokens stay within budget for a normal case | `pack_conformance::pack_tokens_stay_within_budget_for_a_normal_case_accounting_for_headings_joiners_footer` |
| `src/commands/pack.conformance.test.ts` | pack never surfaces meta/friction.md content, even when it exists | `pack_conformance::pack_never_surfaces_meta_friction_md_content_even_when_it_exists` |
| `src/commands/pack.conformance.test.ts` | pack's omitted footer is capped and summarizes the rest instead of listing every record | `pack_conformance::packs_omitted_footer_is_capped_and_summarizes_the_rest_instead_of_listing_every_record` |
| `src/commands/pack.conformance.test.ts` | FRI-001: pack --budget -1 gives the existing validation error | `pack_conformance::fri_001_pack_budget_dash_1_gives_the_existing_validation_error_not_a_raw_parseargs_crash` |
| `src/commands/reflect.conformance.test.ts` | reflect emits a prompt naming friction, journal months, and the ROADMAP rule | `reflect_conformance::reflect_emits_a_prompt_naming_friction_journal_months_and_the_roadmap_rule` |
| `src/commands/reflect.conformance.test.ts` | reflect includes captured friction entries in its digest | `reflect_conformance::reflect_includes_captured_friction_entries_in_its_digest` |
| `src/commands/reflect.conformance.test.ts` | reflect --json returns a versioned envelope with friction and journal coverage | `reflect_conformance::reflect_json_returns_a_versioned_envelope_with_friction_and_journal_coverage` |
| `src/commands/reflect.conformance.test.ts` | reflect --months limits how many recent journal months are listed | `reflect_conformance::reflect_months_limits_how_many_recent_journal_months_are_listed` |
| `src/commands/reflect.conformance.test.ts` | reflect rejects a non-positive --months | `reflect_conformance::reflect_rejects_a_non_positive_months` |
| `src/commands/reflect.conformance.test.ts` | reflect rejects a non-integer --months instead of silently truncating | `reflect_conformance::reflect_rejects_a_non_integer_months_instead_of_silently_truncating` |
| `src/commands/reflect.conformance.test.ts` | reflect rejects a --months with trailing junk instead of silently parsing a prefix | `reflect_conformance::reflect_rejects_a_months_with_trailing_junk_instead_of_silently_parsing_a_prefix` |
| `src/commands/reflect.conformance.test.ts` | FRI-001: reflect --months -1 gives the existing validation error | `reflect_conformance::fri_001_reflect_months_dash_1_gives_the_existing_validation_error_not_a_raw_parseargs_crash` |
| `src/commands/reflect.conformance.test.ts` | reflect performs no writes to the repo (read-only) | `reflect_conformance::reflect_performs_no_writes_to_the_repo_read_only` |
| `src/commands/reflexive-loop.conformance.test.ts` | reflect leaves the entire working tree untouched | `reflexive_loop_conformance::reflect_leaves_the_entire_working_tree_untouched` |
| `src/commands/reflexive-loop.conformance.test.ts` | feedback writes only under .agnosgram/meta/ | `reflexive_loop_conformance::feedback_writes_only_under_agnosgram_meta` |
| `src/commands/show.conformance.test.ts` | show matches an exact record id first | `show_conformance::show_matches_an_exact_record_id_first` |
| `src/commands/show.conformance.test.ts` | show matches a case-insensitive scope tag when no id matches | `show_conformance::show_matches_a_case_insensitive_scope_tag_when_no_id_matches` |
| `src/commands/show.conformance.test.ts` | show matches a type name as a last resort | `show_conformance::show_matches_a_type_name_as_a_last_resort` |
| `src/commands/show.conformance.test.ts` | --type filters the pool before matching | `show_conformance::type_filters_the_pool_before_matching` |
| `src/commands/show.conformance.test.ts` | no match exits 1 and hints known scopes on stderr | `show_conformance::no_match_exits_1_and_hints_known_scopes_on_stderr` |
| `src/commands/show.conformance.test.ts` | --format json prints a uniform flat array | `show_conformance::format_json_prints_a_uniform_flat_array` |
| `src/commands/show.conformance.test.ts` | --format toon renders a tabular header for multiple matches | `show_conformance::format_toon_renders_a_tabular_header_for_multiple_matches` |
| `src/commands/show.conformance.test.ts` | missing topic argument throws a usage error | `show_conformance::missing_topic_argument_throws_a_usage_error` |
| `src/commands/show.conformance.test.ts` | invalid --type throws a usage error | `show_conformance::invalid_type_throws_a_usage_error` |
| `src/commands/show.conformance.test.ts` | FRI-001: show --type -bogus gives the existing validation error | `show_conformance::fri_001_show_type_dash_bogus_gives_the_existing_validation_error_not_a_raw_parseargs_crash` |
| `src/commands/show.conformance.test.ts` | a bare -- terminator: everything after it stays a literal positional | `show_conformance::a_bare_terminator_everything_after_it_stays_a_literal_positional_not_a_rewritten_option` |
| `src/commands/show.conformance.test.ts` | --type -- pitfall: -- is never swallowed as --type's value | `show_conformance::type_dash_dash_pitfall_dash_dash_is_never_swallowed_as_types_value` |
| `src/commands/show.conformance.test.ts` | --format json on no match prints an empty JSON array to stdout and still exits 1 | `show_conformance::format_json_on_no_match_prints_an_empty_json_array_to_stdout_and_still_exits_1` |
| `src/commands/show.conformance.test.ts` | show never surfaces a meta/ friction entry, even by exact id or its scope tag | `show_conformance::show_never_surfaces_a_meta_friction_entry_even_by_exact_id_or_its_scope_tag` |
| `src/commands/show.conformance.test.ts` | --format toon on no match prints an empty TOON array to stdout and still exits 1 | `show_conformance::format_toon_on_no_match_prints_an_empty_toon_array_to_stdout_and_still_exits_1` |

Total: 104 TS cases -> 104 Rust test functions. None dropped.

## Adaptations (not drops)

A handful of TS assertions guarded against a Node-specific failure mode that
has no Rust equivalent, and were narrowed rather than dropped wholesale (the
behavioral assertion in the same test - correct exit code, correct stderr
message - is still fully ported):

- `!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr)` - guarded
  against Node's `parseArgs` leaking a raw JS stack trace or its
  `ERR_PARSE_ARGS` error code into stderr instead of the CLI's own clean
  `UserError` message. The Rust binary has no Node internals to leak, so this
  half of each assertion is meaningless in Rust-only reality; the "clean
  UserError message" half is kept in full in every case it appeared (all
  `FRI-001` cases, `bootstrap`, `init`, `log`, `doctor`, `distill`,
  `feedback`, `advise`, `pack`, `reflect`, `show`).
- Regex assertions (`assert.match`) were reimplemented as explicit substring
  or structural checks (`str::contains`, a small hand-rolled JSON parser in
  `tests/common/mod.rs`, a `looks_like_semver` helper, a `has_exact_line`
  helper for the one `^...$`-anchored case) rather than pulling in a regex
  crate, honoring the zero-dependency constraint for integration tests
  (std only, no dev-dependencies).
- `current_journal_month()` reads the actual `YYYY-MM.md` filename off disk
  instead of recomputing "now" with calendar math (no `chrono` in a
  zero-dependency test crate) - equivalent to the TS test's
  `new Date().toISOString().slice(0, 7)`, but robust to real time rather than
  reimplementing civil-date arithmetic by hand.
