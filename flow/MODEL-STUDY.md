---
type: protocol
tags:
  - domain/ai/agents
  - domain/dev/testing
  - type/reference
  - project/borg
  - status/active
aliases:
  - Flow Model and Stack Study
  - Footnote Model Efficiency Map
date_maj: 2026-11-17
---
# Model, stack and task efficiency study

Up: [[AI-Agents/projects/footnote-framework/FULL-EVALUATION|Full evaluation]] · Related: [[AI-Agents/projects/footnote-framework/FOOTNOTE-FLOW-DESIGN|Integrated product design]]

#domain/ai/agents #domain/dev/testing #type/reference #project/borg #status/active

## Question

Johan proposes testing whether better context/tools make lower-cost models substantially more effective, and whether that combination competes with stronger models on complex work. These are hypotheses, not assumed outcomes. A higher-priced model may finish with fewer calls, while extra context may distract a smaller model; the experiment must permit both results.

The target is a task-dependent map of quality, completion time, repairs and total resources, rather than a universal model ranking. No new model-tier comparison has run. The existing medium-effort single-model calibration and two-session integration proof remain separate evidence.

## Primary question (Johan, 18 November 2026)

Not pass or fail: **time spent and cost** for the same accepted work, on ordinary company tasks, comparing the AI alone, the AI with skills, and the AI with Johan's harness (Footnote/Flow). The expected value of the harness is lower cost per delivery or more work per employee for the same spend. Acceptance stays a gate: a run that is not accepted counts its time and tokens against its setup and yields no delivery. Strong models passing every level (Sonnet 5.5 medium baseline passed L1-L4 v1 and L3/L4 v2) is therefore not a problem for this question.

## Agreed campaign sequence, 17 November 2026

Johan requested increasing task complexity first, followed by higher reasoning effort and then the stronger model configurations. Do not implement adaptive routing or choose a preferred stack before this evidence exists. This sequence supersedes the earlier proposed two-model screen below.

Sol uses `gpt-6.2-sol` (Johan, 18 November 2026: 6.1 is no longer in the catalogue and 6.2 is reported better at the same or lower price). Run in configuration order: `gpt-6-luna` medium, `gpt-6-luna` high, `gpt-6.1-terra` medium, Terra high, `gpt-6.2-sol` medium, Sol high. Keep the same four difficulty levels and all five setups for each configuration: baseline, skills only, Footnote only, Footnote plus skills, and Flow prototype. Exact machine-readable cells are in `evaluation/complexity-ladder-v1.json`.

The four levels are local change, downstream dependencies, persistence plus compatibility, and cross-layer diagnosis plus recovery. Existing [[AI-Agents/projects/footnote-framework/LUNA-PILOT|local]] and [[AI-Agents/projects/footnote-framework/DEPENDENCY-PILOT|dependency]] pilots supply ten exploratory observations. L3 and L4 still require concrete snapshots, independent checks and broken/known-valid controls. More files alone do not establish difficulty. Pin the final tasks, acceptance criteria, skill pack, Footnote binary, governor and runtime before comparing configurations. Preserve and disclose differences in the existing pilots rather than quietly counting them as newly matched runs.

The matrix contains 120 configuration/task/setup cells with one observation each: ten existing exploratory observations and 110 unrun cells. This is a development overview, not 120 independent tasks or a statistically complete reliability assessment. Multi-session cases may require more than one worker launch per cell; account for every agent. No additional repetitions or failed-run replacements are implicitly included.

Use identical limits within each level across configurations and setups. Retain failed, incomplete and infrastructure-affected outcomes. Keep actual input, cached subset, output, elapsed time, start latency, local repair evidence and complete-delivery acceptance separate. Record unknown metrics as unknown. Compare resources per accepted outcome with failed-attempt usage included. Preserve baseline's correct functional candidate but incomplete delivery, and the pytest requirement violation in the Footnote-plus-skills dependency result.

The real token predictor remains deferred; use the measurement-only adapter for every benchmark worker. No stronger-model coordinator, routing optimization or production-integration claim is introduced. The aggregate usage envelope is still unspecified after the earlier quota-constrained pilots. Freeze that envelope before launching the expanded campaign. Fixture and checker preparation can proceed independently.

## Factors

Start with two exact pinned model configurations, one lower-priced and one stronger candidate. Choose exact IDs from authenticated available models, verify current pricing and record reasoning effort, service tier, runtime and tool support. A catalog entry does not prove a capability or a price advantage. Within each model configuration, keep settings identical across stacks. The same effort label is not necessarily equivalent across providers/models. Compare model-plus-runtime systems if adapters differ, and label that limitation rather than attributing everything to the model.

Four stack settings make attribution possible:

1. Normal coding agent with common safety/runtime rules, no curated cards or Footnote integration.
2. The same agent with curated skills only.
3. The same skills plus Footnote Code, Memory and Library, without Flow coordination.
4. The same Footnote stack and skills plus integrated Flow, with one working agent by default.

This extends the earlier three-arm proposal with a separate Footnote contribution. If resources require only three arms, hold skills constant and compare settings 2-4; retain the earlier skills evaluation as separate evidence, not a substitute matched control. Do not add GSD/BMAD/OpenSpec/Spec Kit simultaneously to the first model experiment. They are a later benchmark dimension.

Use the same underlying project files, documentation and initial knowledge snapshot for all arms. Footnote may improve delivery of that information but must not contain the answer/future fix. Learning sequences use independent copies of the same initial memory for each model/stack/repetition; later knowledge is never shared across treatments. Define how the ordinary agent can retain its own task notes too, rather than forbidding baseline memory solely to favour Footnote.

## Task strata

- Local change: useful control for unnecessary overhead.
- Change with downstream consumers: functional fix plus independently checked regression preservation.
- Documentation/version constraint: correct implementation depends on the applicable API or compatibility contract.
- Cross-module diagnosis: localization and repair across a real dependency chain.
- Fresh-session follow-up: preserved decisions and reduced rediscovery, without manual answer leakage.
- Interrupted/resumed work: correct reconciliation, no duplicate effects and no lost acceptance state.

Freeze actual repository snapshots and external checks before confirmation. Project size is a descriptor, not task difficulty. Record affected modules, dependency distance, relevant decision/doc requirements and number of sessions. Development stress fixtures and historical tasks with possible model-training exposure must be labelled. Validate empty/broken/known-valid controls before charging models to a task.

## What each run records

Identity: task/snapshot, initial corpus/memory, model/provider/version/effort/tier, stack/pack/runtime revisions, repetition, arm order, agent/attempt/response IDs, environment and cache conditions.

Time: submission, ready, first useful action, first artifact, external acceptance and terminal outcome; phase spans for context, model calls, tools, checks, repair, routing and recovery. Use same-host monotonic clocks where possible. Native entry time is not TTFT. Missing timing stays null.

Quality: first-pass and final acceptance, independent regression checks, defect severity, missed requirements, dependency breakage, human/blinded-review outcome and abandonment. Infrastructure failures remain a separate category, with their consumed resources recorded.

Effort: internal repair rounds, independent repetitions, model calls, tool calls, agents, peak concurrency, human interventions and active review minutes. Define the initial attempt as attempt 1 and repairs as subsequent attempts. A new study repetition is never used to replace a failed run in the results.

Usage: all input, cached input, output and exposed reasoning subsets, per response/agent/role/attempt and run. Include specialist/model-critic/fallback usage in the appropriate arm. Cache is already inside input and reasoning may already be inside output. Separate common evaluation cost from task execution. No hidden higher-tier coordinator or repair model is allowed in an economy-model arm. Mixed-model escalation is its own later configuration.

Cost: invoice/marginal billing if available; otherwise clearly dated API-equivalent estimates with cache/model/tier rates. Subscription quota, list-price estimates and actual charges are different measures. Include index/ingestion/daemon costs separately as initial setup and amortized repeated use, rather than treating them as free or charging only one unlucky arm. Unknown prices remain null.

Primary comparisons: first-pass/final acceptance under shared caps, complete resource use, time to acceptance, and suite resource cost divided by accepted outcomes. Include failed attempts in that denominator's resource numerator. If nothing is accepted, cost per accepted outcome is undefined. Report unsuccessful terminal times and timeouts separately; fast failures are not efficient deliveries. Successful-only charts must be accompanied by failure rates.

## Stages and expense control

These are proposed local workload sizes, not a power calculation or permission to spend an unspecified monetary amount.

1. **Instrumentation/model preflight:** verify exact model access, source isolation, context/tool compatibility, host timing and complete per-response usage. Make invalid/empty/known-valid controls pass the appropriate evaluator checks. Fix collection defects before multiplying runs.
2. **Development screen:** two models x four stacks x three distinct tasks x one repetition = 24 runs. Choose one local, one dependency and one continuity case. This is directional evidence, not a reliability or significance claim. Use it to estimate run cost and duration and identify broken configurations.
3. **Confirmation:** two models x four stacks x six independent task sequences x three repetitions = 144 runs. A third model makes 216. One task sequence may contain several sessions and repairs, so model-call counts are larger than these run counts. Development tasks are separate from confirmation tasks.
4. **Targeted extension:** add middle-tier models or cross-provider configurations where the confirmation map leaves an unresolved practical choice. If the study is changed after examining outcomes, preserve the exploratory evidence and freeze a new confirmation set. Do not repeatedly sample until a preferred arm wins.

Before model execution, select per-run elapsed/token/repair caps and an aggregate monetary or subscription-usage envelope from the development estimates. Stop at caps and save every outcome. Until real cost data exists, do not promise a dollar total or a completion date. Do not silently truncate the experiment to only successful tasks when the envelope is reached.

Block by task/repetition and balance random order over model x stack settings. Avoid provider quota or local resource contention favouring one arm. Record cache observations; a fresh conversation is not necessarily a cold provider cache. Start with one provider family where practical to reduce runtime/tool confounding, then test portability explicitly.

## The global overview

Use linked views with filters for task stratum, cold/warm state, model configuration and stack. Never blend the earlier parser/reference-governor calibration with the integrated product and call them one series.

- Quality heatmap: model x stack, acceptance counts/rates, with per-task drill-down and sample counts. Unrun/missing cells remain visibly empty.
- Cost-quality frontier: x complete cost per accepted outcome, y acceptance, marker colour stack and shape model. Do not place unknown costs at zero. Show uncertainty and exclusions.
- Time/quality view: x accepted-delivery time, y acceptance; show all-run failures/censoring alongside it.
- Rework view: first-pass success, repair distribution, intervention count and total consumption to final outcome. Keep failures that hit the cap visible.
- Resource breakdown: uncached input, cached input, output, model calls, agent roles, context/tool/check phase time and Footnote setup/daemon work. Overlapping spans are not added into wall time.
- Complexity profile: per-stratum paired differences. The useful result may be skills-only for local changes and integrated Flow for continuity/impact tasks; do not assume this routing policy before data exists.

Default displays show raw points/counts before smooth trend lines. With six tasks, do not present a stable p95 or narrow generalization claims. Repetitions and agents are clustered within tasks, not additional independent task samples. Aggregate repetitions within task before estimating task-level differences. A composite score must not conceal a serious regression.

## Readiness

First requested model-specific screen completed: [[AI-Agents/projects/footnote-framework/LUNA-PILOT|Luna medium calibration]], four fresh delegates across the four stacks on one controlled task. All passed external checks. This is instrumentation/development evidence; no higher-tier matched comparison or confirmation set has run. The initial proposed 24-run development size was reduced to four for this explicitly budget-constrained start.

Already available: source-grounded integration design, isolated real-product two-session loop, reconciled native per-response telemetry, baseline skills pilot and reference governor contracts. Still missing: complete production runtime/Usage run attribution, host submission-to-acceptance timing, frozen independent workloads, exact model configurations and the study envelope. Model catalog discovery is complete for planning; no cross-model agents were launched.

## Sources

- [OpenAI evaluation best practices](https://developers.openai.com/api/docs/guides/evaluation-best-practices), accessed 17 November 2026: task-specific evals, logging, calibrated judging and representative data. No numeric example targets are adopted.
- [OpenAI cost optimization](https://developers.openai.com/api/docs/guides/cost-optimization), accessed 17 November 2026: balance model size, accuracy, request count and token use. This is not evidence of Flow's savings.
- [[AI-Agents/projects/footnote-framework/INTEGRATED-LOOP|Working integration]] and [[AI-Agents/projects/footnote-framework/LIVE-CALIBRATION|Earlier overhead calibration]].
