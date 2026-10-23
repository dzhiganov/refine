# Decisions and why

Each decision links to the evidence in [research/stats-system.md](research/stats-system.md) (§ numbers
refer to it). Where the evidence is weak, it says so.

## Corrections

| Decision | Why |
|---|---|
| Correct every real error, directly, with a short explanation. | Written corrective feedback has a moderate, lasting effect; direct, indirect and metalinguistic feedback perform similarly (Kang & Han 2015; Lim & Renandya 2020; Brown, Liu & Norouzian 2023). Comprehensive direct correction improved accuracy in new texts (Van Beuningen et al. 2012). §3.1 |
| "If it's understandable and correct, leave it." Style preferences go to suggestions. | Matches the global/local error distinction and MERLIN's intelligibility tags; MERLIN's "comprehensible but imprecise" vocabulary tag is the same split (§3.2). Keeps fixes trustworthy and avoids rewriting the learner's voice. |
| Every mistake gets `impedesMeaning: true/false`, shown separately. No severity score. | Global vs local is well established; numeric error-gravity weightings never stabilized (Rifkin & Roberts 1995; Bryant et al. 2023). §3.2 |
| A fixed two-level taxonomy (shared top level, 21 EN / 25 DE subcategories), enforced by the JSON schema. | Mirrors ERRANT (EN) and MERLIN (DE) for checkability. Fine where annotators agree (inflection κ 0.75–0.84), coarse where they don't (lexis κ ≈ 0.37). §3.4, §4.2 |
| Contrastive rules in the prompt for close pairs (case vs gender, tense vs form, word choice vs collocation, preposition vs collocation). | Labels drift most between confusable categories; the research recommends exactly these pairs. §3.9 |
| Suggestions: at most 3, typed collocation / phrase / synonym, at most one CEFR level above the learner. Over-ambitious ones are dropped in code. | Formulaic language matters and is slow to acquire (Nesselhauf 2003; Boers & Lindstromberg 2012). The level cap keeps suggestions learnable; i+1 is a heuristic, not a law, so it's a cap, not a promise. §3.6, §3.7 |
| Tone presets with formality and a short-forms policy. | Register errors are real errors only relative to the setting; genre also moderates feedback effects (Kang & Han 2015). §2 |

## Reliability

| Decision | Why |
|---|---|
| Strict JSON schema (`response_format: json_schema, strict`), falling back to JSON mode, then to plain text with the schema in the prompt. The mode that worked is remembered per model. | Works with OpenRouter, OpenAI, Ollama and LM Studio, including models without structured outputs. |
| Mistakes are listed before `fixedText` in the schema. | Generation follows key order; the model commits to each correction before writing the whole text, which keeps the two consistent. |
| Validation in code, one repair round, then drop what's still invalid with a warning. | `original` must be verbatim in the input, corrections must appear in the fixed text, categories must be in the enum, a changed text needs listed mistakes. A silent rewrite is never shown as a fix. |
| Prompt-injection guard: the text is wrapped and declared as data. | A pasted text can contain instructions; the smoke test covers it. |

## Stats

| Decision | Why |
|---|---|
| Rates are errors per 100 words over word windows (1,500 words, at least 500 to show a number); texts under 20 words count for patterns, not rates. | Raw counts track how much you write; per-text rates on short messages are noise. §4.1 |
| "Improving"/"Rising" only when an exact Poisson rate-ratio test gives p < 0.05; otherwise "Steady". | Prevents celebrating or worrying about noise. §4.1 |
| A pattern recurs only with errors in ≥2 texts on ≥2 days within 30 days; one-offs are archived after 30 days. | Recurrence across occasions is the practical marker of a systematic error vs a slip (Corder 1967; Schmidt 1990). §3.3, §4.3 |
| Watch list of at most 3 patterns; the model reports correct uses and errors for them. | Attention is limited; without opportunity counts "no errors lately" can't be told from "stopped using it". §3.1, §4.3 |
| Mastered = ≥90% correct in 3 consecutive opportunity texts, ≥6 uses, 14 days without the error. Silence never counts as mastery (dormant instead). | Brown's (1973) 90%-in-3-samples criterion; the 14-day gap stands in for a delayed post-test. **Borrowed from L1 research; the 75% "improving" threshold and 14 days are design choices.** All thresholds are published in the stats JSON. §4.3, §6 |
| Reviews on a 1-3-7-14-30-60-day ladder; a recurrence resets it. Review = fix your own past sentence, self-graded. | Spacing and retrieval are among the most robust findings in learning science (Cepeda et al. 2006, 2008; Karpicke & Roediger 2008). **Applying them to grammar in free writing is an extrapolation.** §3.5 |
| Suggestion uptake: a suggested phrase later appears in your own uncorrected text. | Direct evidence a formulaic sequence was picked up. String match, not lemma match: may miss inflected reuse. §3.6 |
| Not shown: total mistake counts, a score, a CEFR estimate, comparisons, breakable streaks. "Weeks with writing" is a soft count. | Feedback aimed at the self rather than the task can hurt (Kluger & DeNisi 1996); broken streaks lead people to quit (Silverman & Barasch 2022); missing a day doesn't hurt habits (Lally et al. 2010). §3.8, §4.5 |
| Every number is computed in code. The model writes only the summary, from the finished stats, and every number it quotes is checked against them (one repair round, then flagged). | LLMs are unreliable at arithmetic; the numbers must be explainable. §4.4 |

## Deviations from the research doc

- **Pattern mapping at check time, not in a separate idle pass.** The doc suggests the LLM map free
  "pattern hints" to keys during idle time. Instead the check prompt includes the learner's known
  pattern labels and asks the model to reuse one when it's the same issue. Same effect, no extra
  call, and the key is stable from the first check. A later consolidation pass for near-duplicate
  labels is possible.
- **Known simplifications:** with little writing, the two comparison windows split the available
  words in half instead of falling back to 30-day windows; thresholds are fixed in code (but
  published in the stats JSON); word counts include URLs and code. Days and weeks use the
  computer's current UTC offset.
- **Not yet built:** the κ label-consistency audit (§3.9), MTLD lexical diversity (opt-in, §3.6),
  FSRS instead of the fixed ladder (§3.5). The stats JSON has room for them.
