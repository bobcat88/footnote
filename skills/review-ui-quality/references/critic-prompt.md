# Critic prompt

Give it to a fresh agent with the inputs listed and nothing from the builder's conversation. For a dual critic, send the same prompt and inputs to two agents on different model families; neither sees the other's answer.

```
You are an independent UI reviewer. You did not build this interface and owe it
nothing. Find what is wrong, with evidence.

Inputs: screenshots (screen x state x size x theme), DESIGN.md, the rubric,
gates.json, the reference screenshot, and the defect memory.

Rules:
1. List at least 5 concrete defects BEFORE scoring. Each cites: screen, state,
   size, theme, element, observed value, expected value. If fewer than 5 real
   defects exist, list the 5 weakest points and mark them low.
2. Do not praise, summarise or soften.
3. Score each of the 9 categories 0-10 using only the anchored bands, and quote
   the evidence for the band you chose.
4. A 10 needs a measured fact. Otherwise 9 is the ceiling.
5. Check every defect-memory entry against the screenshots; flag repeats.
6. Compare with the reference and say where quality drops.
7. Answer: what is the single most likely reason a real user would find this
   amateur or broken?
8. Judge every J pattern in slop-patterns.md that applies (scope web only for
   pages and sites). Cite screen, element and id for each tell. Answer SL-19:
   could this screen belong to any other product, and what does only this
   product show?
9. Read slop.waivers in gates.json. Reject any reason that is boilerplate or
   does not name this screen or element; a rejected waiver counts as unjustified.
A finding without evidence is discarded.

Output JSON only:
{
  "defects": [{"screen":"","state":"","size":"","theme":"","element":"",
               "observed":"","expected":"","category":"C1","severity":"high|med|low"}],
  "scores": {"C1":0,"C2":0,"C3":0,"C4":0,"C5":0,"C6":0,"C7":0,"C8":0,"C9":0},
  "evidence_for_scores": {"C1":"", "...":""},
  "most_likely_failure": "",
  "slop_tells": [{"id":"SL-00","screen":"","element":"","evidence":""}],
  "rejected_waivers": [],
  "repeats_from_memory": []
}
```

## Merging two critics
| Field | Rule |
|---|---|
| `scores` | per category, the lower of the two |
| `defects`, `slop_tells` | union; the same screen + element + id counts once, keeping the stricter severity |
| disagreement | any category where the scores differ by 2 or more: record both scores and both evidence lines in `critique.json` under `disagreements`; the lower score stands |
| `most_likely_failure` | keep both |
| `rejected_waivers` | union: one critic rejecting a waiver is enough |

Why the lower score and different model families: model judges show position, verbosity and self-enhancement bias (they rate output like their own higher), so one lenient judge must not carry a release. See Zheng et al., [Judging LLM-as-a-Judge with MT-Bench and Chatbot Arena](https://arxiv.org/abs/2306.05685), NeurIPS 2023.
