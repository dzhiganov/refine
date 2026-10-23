# Refine: stats and progress system, research and design

Status: research draft, 2026-10-23. Scope: what Refine should compute, store and show about a user's mistakes, suggestions and progress. It also covers how strong the evidence behind each decision is.

---

## 1. Summary of recommendations

1. **Keep correcting everything that matters, but have the stats focus on a few patterns.** Written corrective feedback (WCF) has a moderate effect that lasts over time. Direct correction works at least as well as other feedback types. Comprehensive correction also works. Attention is limited, though, so the stats and reminders should push no more than 3 "watch" patterns at a time. → [§3.1](#31-written-corrective-feedback)
2. **Rank by recurrence, not by raw count.** A pattern that shows up across separate texts and days points to a rule in the learner's interlanguage, not a slip. That is the unit worth noticing and the unit that risks fossilizing. → [§3.3](#33-noticing-recurrence-fossilization)
3. **Mark every error `impedes_meaning: true/false` (global vs local) and keep the two apart in the stats.** Don't build a weighted "gravity score": error-gravity research has not produced a stable universal hierarchy. → [§3.2](#32-error-gravity-global-vs-local-treatable-vs-untreatable)
4. **Use a two-level taxonomy with one shared top level for all languages:** `grammar`, `lexis`, `orthography`, `register`, `discourse`, plus `other`. Subcategories are fine-grained where human agreement is high (morphology, agreement, word order) and coarse where it is low (lexis, style). EN has 21 subcategories and DE has 25. → [§3.4](#34-error-taxonomies-and-labeling-reliability), [§4.2](#42-error-taxonomy)
5. **Never count suggestions as errors.** Track them as a separate "naturalness" stream, and measure *uptake*: whether the user later reuses a suggested phrase on their own. → [§3.6](#36-formulaic-language-collocations-lexical-diversity)
6. **Always normalize: errors per 100 words, over word-based windows of at least 500 words, split by language and by tone.** Raw counts mostly track how much the user writes. → [§4.1](#41-metrics)
7. **Report a trend only when it is bigger than the noise.** Use a Poisson rate-ratio test between windows. Otherwise say "stable". → [§4.1](#41-metrics)
8. **An error pattern moves through new → recurring → improving → mastered, plus `relapsed` and `dormant`.** "Mastered" means correct in at least 90% of obligatory contexts across at least 3 consecutive texts that had the chance, over at least 14 days. The 90%/3-sample rule is the classic acquisition criterion. → [§4.3](#43-error-pattern-lifecycle)
9. **Count opportunities, not just errors.** For watched patterns, the correction call reports correct uses as well as errors. Without that, "no errors lately" can't be told apart from "avoided the structure". → [§4.3](#43-error-pattern-lifecycle)
10. **Schedule "watch out for X" reminders on an expanding ladder (1, 3, 7, 14, 30, 60 days).** A recurrence resets the ladder. Micro-reviews should be retrieval tasks: "fix your own sentence", not "reread the rule". Move to FSRS once there is enough data. → [§3.5](#35-spaced-repetition-and-retrieval-practice)
11. **Tag suggestions with an estimated CEFR level and pitch them at most one level above the user's self-declared level per language.** Don't show the user a CEFR level estimate. → [§3.7](#37-cefr-profiles-and-i1)
12. **Show improvement trends and "patterns mastered".** Don't show totals of mistakes, scores out of 10, peer comparison or loss-framed streaks. → [§3.8](#38-motivation-and-dashboards), [§4.5](#45-what-not-to-show)
13. **Show lexical diversity (MTLD) only as an optional, long-window, same-tone trend on texts of at least 100 tokens.** Never use it as a headline number. → [§3.6](#36-formulaic-language-collocations-lexical-diversity)
14. **Compute every number deterministically.** The LLM only labels errors (fixed enum), maps free-text pattern hints onto existing pattern keys, and writes a short prose summary from the finished JSON. → [§4.4](#44-deterministic-vs-llm), [§3.9](#39-keeping-llm-labels-consistent)
15. **Enforce the label enum with a JSON schema, give few-shot examples for the confusable pairs, and measure label consistency** by re-labeling a sample. Target κ ≥ 0.6 at subcategory level. → [§3.9](#39-keeping-llm-labels-consistent)

---

## 2. Design constraints from the product

- The corrections are deliberately minimal ("if it's understandable, it's fine"). So nearly every recorded error is a real grammar error or an error that distorts meaning. Stylistic preferences arrive as *suggestions*. This lines up with the error/suggestion split in the research below.
- Texts are short and come in very different genres (Slack vs formal email). Genre moderates feedback effects (Kang & Han 2015), and it changes both error rates and lexical measures. Every rate is therefore split by tone.
- One adult user, self-directed, wanting slow and steady progress. Autonomy and a sense of competence matter more than gamified pressure (§3.8).

---

## 3. Evidence

### 3.1 Written corrective feedback

**What the research says.** Truscott (1996) argued that grammar correction in L2 writing is ineffective and should be abandoned. His 2007 meta-analysis put the best estimate of the effect near zero or slightly negative. Ferris (1999) replied that correction helps when it is "selective, prioritized, and clear", and that the earlier studies were too heterogeneous to support Truscott's conclusion. Ferris (2004) set out the research agenda that followed. Since then the controlled evidence has mostly favored WCF:

- **Kang & Han (2015)**, a meta-analysis of 21 studies, found that WCF improves grammatical accuracy, with the effect moderated by proficiency, setting and genre.
- **Lim & Renandya (2020)**, a meta-analysis of 35 studies, found a moderate overall effect. Direct feedback beat indirect feedback numerically, but the difference was not significant. Proficiency was the strongest moderator.
- **Brown, Liu & Norouzian (2023)**, a Bayesian meta-analysis of 52 controlled studies, found "robust evidence of the durability of moderate effectiveness of WCF over time". Direct, indirect and metalinguistic feedback (the types in Ellis's 2009 typology) "all yield[ed] similar effect sizes".
- **Van Beuningen, De Jong & Kuiken (2012)**, N = 268, found that *comprehensive* direct and indirect CF both improved accuracy in new texts at 1 and 4 weeks. Only direct CF improved *grammatical* accuracy in new writing, and correction did not make learners simplify their writing.
- **Focused vs comprehensive.** Focused WCF on two article functions beat no feedback across a 10-month study (Bitchener & Knoch 2008/2009, N = 52). Sheen (2007) found that focused direct metalinguistic feedback held up better at the delayed post-test than direct correction alone. But Shintani, Ellis & Suzuki (2014) found direct correction more durable than metalinguistic explanation, and found that a chance to revise boosted the effect. A replication of Bitchener & Knoch (Ekiert & di Gennaro 2019) partly confirmed the gains, but found that highly explicit focused feedback *hurt* the article uses it did not target.

**Strength.** The meta-analytic evidence is moderate and fairly consistent: WCF works, and the effect lasts. Which *type* works best is contested and probably small. Most studies are classroom studies of students on a narrow set of targets, mostly English articles, so generalizing to an adult professional's own texts is a reasonable extrapolation, not a direct finding.

**Design implications.** Keep comprehensive, direct correction with a short explanation. It is supported, and it is what users expect. Use the stats layer to *focus*: at most 3 watched patterns, chosen by recurrence. Encourage revision ("fix your own sentence" micro-reviews, §3.5), since revision improved retention in Shintani et al. (2014).

### 3.2 Error gravity: global vs local, treatable vs untreatable

**What the research says.** Burt & Kiparsky (1972) and Burt (1975) separated *global* errors, which affect overall sentence organization and cause misreading, from *local* errors, which affect one constituent and rarely block understanding. The judgments came from native speakers rating comprehensibility. Error-gravity surveys (e.g. Vann, Meyer & Lorenz 1984, 164 faculty) found that readers do rank errors, but the hierarchies depend on the raters. Rifkin & Roberts (1995) reviewed 28 error-gravity studies and concluded that the field had "only skimmed the surface" and that judgments are shaped by extralinguistic factors. A 2023 GEC survey still calls error difficulty or gravity "an unsolved problem" (Bryant et al. 2023). MERLIN, the German/Italian/Czech CEFR learner corpus, has explicit *intelligibility* tags at sentence and text level. It also has a vocabulary tag for expressions that are "comprehensible but imprecise", which is essentially Refine's split between errors and suggestions (Wisniewski et al. 2014).

Ferris separates *treatable* errors, which are rule-governed (verb tense/form, subject–verb agreement, articles, noun endings, pronouns), from *untreatable* ones, which are idiosyncratic or lexical (word choice, idiomatic phrasing, some sentence structure). Empirical support is mixed: in some studies article and noun-form errors did not respond to coded feedback, which challenges the neat split.

**Strength.** The global/local idea and the comprehensibility criterion are well established. Any *numeric* weighting of error types has weak support.

**Design implications.**
- The "if it's understandable, it's fine" rule is defensible: it matches the global/local and intelligibility tradition.
- Each error gets a boolean `impedes_meaning`. The stats report global errors separately ("2 sentences were hard to understand this month"). There is no weighted composite.
- Each subcategory carries a static `kind: rule | item` attribute derived from Ferris's distinction. Rule-type patterns get a short rule explanation in reminders. Item-type patterns (collocations, prepositions after specific verbs) get the *specific item* to remember, not a rule.

### 3.3 Noticing, recurrence, fossilization

**What the research says.** Schmidt (1990) argued that noticing is "the necessary and sufficient condition for converting input to intake", and that attention may be necessary for adults learning redundant grammatical features. Corder (1967) separated systematic *errors*, which reflect the learner's current rule system, from random *mistakes* or slips. Selinker (1972) and later Han (2004) described *fossilization*: interlanguage forms that persist despite exposure.

**Strength.** The noticing hypothesis is influential but debated in its strong form; the weaker claim that attention helps is widely accepted. The error/mistake distinction is a classic heuristic, not something measured precisely.

**Design implications.** A form that recurs across *separate texts on separate days* is the operational stand-in for a systematic error. That is what the stats should surface for noticing. A one-off in a single text is treated as a probable slip and never promoted to the watch list. Raw frequency misleads: one long email can contain five article errors from a single slip pattern.

### 3.4 Error taxonomies and labeling reliability

**What exists.**
- **ERRANT** (Bryant, Felice & Briscoe 2017) has 25 main categories based mostly on part of speech (DET, PREP, VERB:TENSE, VERB:SVA, NOUN:NUM, WO, SPELL, ORTH, PUNCT, MORPH, OTHER...), each crossed with Missing/Replacement/Unnecessary. In the authors' evaluation, five experts rated at least 95% of the predicted types "Good" or "Acceptable" (κ_free = 0.72). ERRANT is rule-based and English-only.
- **NUCLE** (Dahlmeier, Ng & Wu 2013) has 27 categories in 13 groups. Human agreement among trained annotators was only κ ≈ 0.39 for *whether* something is an error and κ ≈ 0.55 for the *error type*.
- **MERLIN** (Wisniewski et al. 2014; Boyd et al. 2014) covers German, Czech and Italian, A1–C1. Its top-level groups are grammar, orthography, vocabulary, coherence/cohesion, sociolinguistic appropriateness, pragmatics and intelligibility. Its German-relevant grammar tags include word order (main/subordinate clause), case, gender, number, adjective/noun/verb inflection, article, preposition, valency, reflexives, negation and tense/mood. **Falko** uses competing target hypotheses for German (Reznicek, Lüdeling & Hirschmann 2013).
- **Agreement by category.** CzeSL (Štindlová et al. 2011) reports high agreement for inflection errors (κ 0.75–0.84) and word boundaries (0.72), moderate agreement for agreement and syntactic dependency errors (0.43–0.54), and low agreement for "lexical or phraseology" errors (0.37) and colloquial style (0.25–0.42). Correcting is inherently subjective, and one sentence often has several valid corrections (Bryant & Ng 2015).
- **LLMs.** GPT-4 prompted naively explained only 40.6% of grammar errors. With structured edit extraction first, 93.9% (German) and 92.2% (English) of errors were correctly detected and explained (Song et al. 2024). For simpler annotation tasks, ChatGPT's inter-coder agreement beat crowd workers and trained annotators (Gilardi et al. 2023). I found **no peer-reviewed study that directly measures LLM agreement on error-type labels against a fixed learner-error taxonomy**. That is an evidence gap.

**Strength.** It is well documented that labeling reliability falls from form-level categories to lexical and stylistic ones. Any particular taxonomy size is a judgment call.

**Design implications.**
- Use a shared top level modeled on MERLIN's groups, collapsed to 5 + `other`, so cross-language stats work.
- Subcategories map to ERRANT (EN) and MERLIN (DE) so the scheme can be checked against established practice. Grammar is fine-grained, where agreement is high and the pedagogy is distinct. Lexis, register and discourse stay coarse, where even humans disagree.
- The final size is about 20–25 subcategories per language: ERRANT/NUCLE scale, but with lexis collapsed. A three-level scheme like MERLIN's full tag set would be too fine for consistent LLM labeling and too fine for one user's data to fill.

### 3.5 Spaced repetition and retrieval practice

**What the research says.** Cepeda et al. (2006) meta-analyzed 839 assessments across 317 experiments. Spacing beats massing, and the best gap grows with the retention interval. Cepeda et al. (2008), N > 1,350, found the best gap to be about 20–40% of a 1-week retention interval and about 5–10% of a 1-year interval. Testing beats restudy: once an item is learned, repeated retrieval had large effects on delayed recall while repeated study had none, and learners could not predict their own retention (Karpicke & Roediger 2008; Roediger & Karpicke 2006). The shape of Ebbinghaus's forgetting curve was replicated by Murre & Dros (2015). Applied schedulers include Duolingo's half-life regression, which cut recall-prediction error by more than 45% and raised daily engagement by 12% (Settles & Meeder 2016), and FSRS (Ye, Su & Cao 2022). FSRS models difficulty, stability and retrievability. Its maintainers report 20–30% fewer reviews than SM-2 for the same retention (a project wiki claim, not peer-reviewed).

**Strength.** Spacing and retrieval are among the most robust findings in learning science, *for recall of facts and vocabulary*. Applying them to accurate *use of grammar in free writing* is an extrapolation. Corrective-feedback studies with delayed post-tests (§3.1) only suggest that durability is achievable.

**Design implications.**
- Watched patterns get reminders on an expanding ladder: 1, 3, 7, 14, 30, 60 days. A recurrence in real writing resets the pattern to step 1. A correct use in real writing advances it one step. This is a Leitner-style approximation of the Cepeda ratios. FSRS can replace it later, treating each real-writing opportunity as a review graded pass or fail.
- A reminder is ideally a **retrieval micro-review**: show one of the user's own erroneous sentences with the error highlighted and ask them to fix it before revealing the fix. A passive "remember: dative after *mit*" banner is the fallback.
- Real writing counts as a review. That makes the schedule mostly self-maintaining.

### 3.6 Formulaic language, collocations, lexical diversity

**What the research says.** Nesselhauf (2003, 2005) studied advanced German-speaking learners of English. Verb–noun collocations were a persistent problem, and L1 influence was "far beyond what earlier... studies have predicted". Wray (2002) set out the role of formulaic sequences in native-like selection. Boers & Lindstromberg (2012) reviewed interventions: learners gain a lot from formulaic sequences but are "slow to close the gap". Treatments that help are drawing attention to sequences, stimulating lookups, and helping learners commit specific sequences to memory.

On lexical diversity: type–token ratio depends on text length. MTLD is the most length-robust of the common measures (McCarthy & Jarvis 2010). Koizumi & In'nami (2012) found MTLD least affected by length but still affected below about 100 tokens. Jarvis (2013) argues that lexical diversity is multidimensional and that current indices mostly measure repetition rate.

**Strength.** The case for formulaic language is strong. Lexical diversity as a *progress* metric for one person's short, mixed-genre texts is weak: values swing with genre, topic and length.

**Design implications.**
- The "more natural" suggestions are well grounded. Tag each one `naturalness.collocation | naturalness.phrase | variety.synonym`.
- The main suggestion metric is **uptake**: a suggested phrase later appears in the user's own text, lemma-matched, not as a correction. This is direct evidence that a formulaic sequence was learned.
- Show MTLD only as an opt-in monthly trend, per language and per tone, on texts of at least 100 tokens, with a caveat. Never compare across tones.

### 3.7 CEFR, profiles and i+1

**What exists.** The English Grammar Profile derives more than 1,200 CEFR-levelled grammar competence statements from the Cambridge Learner Corpus (O'Keeffe & Mark 2017). The English Vocabulary Profile levels about 7,000 headwords plus phrases, phrasal verbs and idioms, using the same corpus (Capel 2012). For German, *Profile deutsch* (Glaboniat et al.; Council of Europe initiative, A/DE/CH) gives level-tagged can-do statements, grammar and vocabulary from A1 to C2. Krashen's i+1 (1985) motivates pitching input slightly above the current level.

**Strength.** EGP and EVP are corpus-based and solid as *reference* descriptions. i+1 is a widely cited idea but hard to test empirically, so don't cite it as proof. An LLM's guess at an item's CEFR level is unvalidated.

**Design implications.** The user declares a level per language. The correction call tags each suggestion with an estimated `cefr` level, and the app hides suggestions more than one level above the declared one. That keeps suggestions learnable without claiming precision. Don't display an estimated CEFR level for the user: error rates don't map onto CEFR in any validated way.

### 3.8 Motivation and dashboards

**What the research says.** Self-determination theory says motivation depends on competence, autonomy and relatedness (Ryan & Deci 2000). Feedback improves performance on average (d = .41), but more than a third of feedback interventions *reduced* performance. Effectiveness falls as feedback moves attention toward the self and away from the task (Kluger & DeNisi 1996; see also Hattie & Timperley 2007). Reviews of learning-analytics dashboards find them rarely grounded in learning theory and rarely shown to improve learning (Bodily & Verbert 2017; Matcha et al. 2020). Peer comparison "was not perceived positively by all learners" (Jivet et al. 2018), and awareness alone does not lead to action (Jivet et al. 2017). Gamification has small positive effects (g = .25–.49), but motivational effects are unstable under rigorous designs (Sailer & Homner 2020). When a logged streak breaks, people often drop the activity (Silverman & Barasch 2022). In habit formation, missing a single opportunity "did not materially affect" the process (Lally et al. 2010).

**Strength.** The theory (SDT, feedback-intervention theory) is strong. Direct evidence on dashboards is thin. Evidence on streaks is weak and mostly not from education; Duolingo's own claims are not independently verified.

**Design implications.** Frame stats around the task and improvement ("article errors: down from 2.1 to 0.9 per 100 words"), not the self ("you made 143 mistakes"). Give autonomy: the user can pin or unpin watched patterns. Show "weeks with writing" as a soft count if anything, never a breakable daily streak. No peer comparison.

### 3.9 Keeping LLM labels consistent

Evidence here is mostly engineering practice; there is little peer-reviewed work specific to error typing (§3.4).
- **Closed enum through JSON schema / structured output.** An unknown ID is repaired to `grammar.other` when the top level is `grammar`, and to top-level `other` otherwise. It is never stored as free text.
- **One definition line and 1–2 contrastive examples for each confusable pair** in the system prompt: `grammar.case` vs `grammar.gender`, `grammar.verb_tense` vs `grammar.verb_form`, `lexis.word_choice` vs `lexis.collocation`, `grammar.preposition` vs `lexis.collocation`.
- **Separate the pattern from the label.** The label is the enum. The `pattern_hint` is a short, free, canonical phrase ("dative after mit"). During idle time an LLM maps new hints onto existing `pattern_key`s or creates a new one. The mapping is stored, so later counts are deterministic.
- **Measure consistency.** Re-label a random 5% of stored errors with a fresh call and compute Cohen's κ at top and subcategory level. Humans reach about 0.55 on error type (NUCLE). Aim for at least 0.6, "substantial" on the Landis & Koch (1977) scale. If a subcategory falls below that, merge it upward.
- A self-reported `confidence` field may be stored as a soft flag, but I found no evidence that it is calibrated for this task, so don't use it to filter stats.

---

## 4. Proposed stats design

### 4.1 Metrics

Notation: for a set of texts T, `W(T)` is the word count (whitespace tokens excluding URLs, code and emoji) and `E(T)` is the error count. Everything below is computed per **language**, and per **tone** where stated.

| Metric | Formula | Notes |
|---|---|---|
| Error rate | `ER = 100 × E(T) / W(T)` | Headline accuracy metric. Shown only when `W(T) ≥ 500`. |
| Global error rate | `GER = 100 × E_global(T) / W(T)` | Errors with `impedes_meaning = true`. Shown separately. |
| Category rate | `CR_c = 100 × E_c(T) / W(T)` | Per top level and per subcategory. |
| Category share | `E_c / E` | Used for "where your errors are", never as a judgement. |
| Trend | `RR = ER_recent / ER_prior` over two adjacent **word windows** (default: the most recent 1,500 words vs the 1,500 before). | Label `improving` if the exact Poisson rate-ratio test (conditional binomial on E_recent given E_recent + E_prior, weighted by words) gives p < 0.05 and RR < 1; `worsening` if p < 0.05 and RR > 1; otherwise `stable`. Fall back to 30-day windows if writing volume is too low. |
| Recurrence (pattern p) | `texts_p` = distinct texts with p; `days_p` = distinct calendar days with p | Used for lifecycle promotion. |
| Pattern accuracy | `acc_p(text) = correct_p / (correct_p + errors_p)` | Only for watched patterns, where the correction call reports correct uses (obligatory contexts). |
| Suggestion uptake | `uptake = phrases reused unprompted / phrases suggested` (rolling 90 days) | Reuse = lemma match in a later user text, before correction. |
| Lexical diversity | MTLD per text (≥ 100 tokens), monthly median per language × tone | Opt-in, details view only. |

Why word windows rather than calendar windows: writing volume varies, and per-text rates on 30-word Slack messages are noisy. A window of at least 500 words gives about 5–25 expected errors at typical intermediate rates, which is enough for a rough Poisson comparison. That calibration is my own estimate, not taken from a study.

Excluded from rates: texts under 20 words (they still feed patterns), and suggestions (never errors).

### 4.2 Error taxonomy

The top level is shared across languages. IDs are `top.sub`. `kind` is `rule` or `item` (Ferris treatability, §3.2). "Maps to" points to the external scheme for validation.

**Shared top level**

| Top level | Meaning | MERLIN group | ERRANT range |
|---|---|---|---|
| `grammar` | Morphosyntax: inflection, agreement, word order, function words | Grammar | DET, PREP, PRON, VERB:*, NOUN:NUM/INFL, ADJ:FORM, WO, CONJ |
| `lexis` | Word choice, collocation, word formation | Vocabulary | NOUN/VERB/ADJ/ADV (replacements), MORPH, OTHER |
| `orthography` | Spelling, capitalization, punctuation, word boundaries | Orthography | SPELL, ORTH, PUNCT, CONTR |
| `register` | Mismatch with the chosen tone or form of address (only when it is an error, not a preference) | Sociolinguistic appropriateness | none |
| `discourse` | Connectors and reference that distort logic or make a referent unclear | Coherence/cohesion | none (partly CONJ) |
| `other` | Unclassifiable | none | OTHER/UNK |

**English (21 subcategories)**

| ID | Kind | Covers | ERRANT |
|---|---|---|---|
| `grammar.article` | rule | a/an/the/zero article | DET |
| `grammar.preposition` | item | wrong, missing or extra preposition | PREP |
| `grammar.verb_tense` | rule | tense, aspect, modal, passive choice | VERB:TENSE |
| `grammar.verb_form` | rule | infinitive/gerund/participle, irregular forms | VERB:FORM, VERB:INFL |
| `grammar.agreement` | rule | subject–verb agreement | VERB:SVA |
| `grammar.noun_number` | rule | singular/plural, countability (*informations*) | NOUN:NUM, NOUN:INFL |
| `grammar.pronoun` | rule | form, case, reflexive | PRON |
| `grammar.adj_adv` | rule | comparative/superlative, adjective vs adverb | ADJ:FORM, MORPH |
| `grammar.word_order` | rule | constituent order, question order, adverb placement | WO |
| `grammar.sentence_structure` | rule | fragments, run-ons, missing subject or verb, broken conditional | (multi-token) |
| `grammar.other` | none | any other grammar error | none |
| `lexis.word_choice` | item | wrong word that changes meaning, false friends | NOUN/VERB/ADJ/ADV |
| `lexis.collocation` | item | non-native word combinations and fixed phrases (*make a photo*) | OTHER |
| `lexis.word_form` | rule | derivation (*success* vs *successful*) | MORPH |
| `orthography.spelling` | item | misspelling | SPELL |
| `orthography.capitalization` | rule | proper nouns, sentence start, *I* | ORTH |
| `orthography.punctuation` | rule | only when it distorts meaning or is clearly wrong | PUNCT |
| `orthography.word_boundary` | rule | *alot*, *every day* vs *everyday* | ORTH |
| `register.formality` | rule | too informal or formal for the chosen tone | none |
| `discourse.connector` | rule | wrong linker that changes the logic (*therefore* vs *however*) | CONJ |
| `discourse.reference` | rule | unclear or wrong pronoun reference across sentences | none |

**German (25 subcategories)**

| ID | Kind | Covers | MERLIN tag |
|---|---|---|---|
| `grammar.article` | rule | presence/absence, definite vs indefinite | G_Art |
| `grammar.case` | rule | wrong case of article, pronoun or noun for its role or preposition | G_Morphol_case |
| `grammar.gender` | item | wrong noun gender (shows up in the article or adjective ending regardless of case) | G_Morphol_gend |
| `grammar.adjective_ending` | rule | adjective declension | G_Inflect_adj |
| `grammar.noun_number` | item | plural formation | G_Inflect_noun, G_Morphol_numb |
| `grammar.verb_form` | rule | conjugation, Partizip II, haben/sein auxiliary | G_Inflect_verb, G_Verb_compl |
| `grammar.verb_tense` | rule | tense/mood choice (Perfekt/Präteritum, Konjunktiv II) | G_Verb_tns, G_Verb_md |
| `grammar.agreement` | rule | subject–verb agreement | G_Agr |
| `grammar.word_order` | rule | V2, verb-final in subordinate clauses, separable prefix, *zu* position | G_Wo_womaincl, G_Wo_wosubcl |
| `grammar.preposition` | item | wrong or missing preposition, incl. prepositional verbs | G_Prep |
| `grammar.valency` | item | missing or extra object, reflexive pronoun | G_Valency, G_Refl |
| `grammar.pronoun` | rule | pronoun form other than case (e.g. *es*, relative pronouns) | G_Morphol |
| `grammar.negation` | rule | *nicht* vs *kein*, position of *nicht* | G_Neg_neggen |
| `grammar.sentence_structure` | rule | fragments, missing finite verb, broken clause linkage | (multi-token) |
| `grammar.other` | none | any other grammar error | none |
| `lexis.word_choice` | item | wrong word that changes meaning, false friends | V_Sem_denot |
| `lexis.collocation` | item | non-native combinations and fixed phrases (*eine Entscheidung machen*) | V_FS_colloc, V_FS_idiom |
| `lexis.word_form` | rule | derivation and compounding | V_Wordform_deriv/_comp |
| `orthography.spelling` | item | misspelling, umlaut/ß | O_Graph |
| `orthography.capitalization` | rule | noun capitalization, *Sie/Ihnen* | O_Capit |
| `orthography.punctuation` | rule | comma before subordinate or infinitive clause when clearly required | O_Punct |
| `orthography.word_boundary` | rule | separate vs joined spelling | O_Wordbd |
| `register.formality` | rule | tone mismatch | S_Form_gen |
| `register.address` | rule | du/Sie mix or inconsistency, wrong greeting or closing | S_Form_addr |
| `discourse.connector` | rule | wrong connector (*deshalb* vs *trotzdem*), incl. the word-order effect of the connector when the choice itself is wrong | C_Coh |

`discourse.reference` (EN) and a DE equivalent can be added if the data shows they are needed; MERLIN has C_Coh_ref. Rule for `case` vs `gender`, to go in the prompt: if the article or ending would be wrong in *every* case for this noun (*das Tisch*), use `gender`. If it would be right for another case (*mit der Mann*), use `case`.

### 4.3 Error-pattern lifecycle

**Unit.** A *pattern* is `(language, subcategory, pattern_key)`, e.g. `(de, grammar.case, "dative after mit")`. Only patterns get a lifecycle; subcategories just aggregate.

**Opportunities.** When a pattern is `recurring` or `improving` and on the watch list, its key goes into the correction prompt. The model returns `{pattern_key, correct: n, errors: m}` for that text: suppliance in obligatory contexts, the method behind the classic acquisition criterion. This is the only way to tell "improving" from "avoiding".

| State | Enter when | Justification |
|---|---|---|
| `new` | First error with this pattern_key | One occurrence may be a slip (Corder 1967) |
| `recurring` | Errors in ≥ 2 distinct texts on ≥ 2 distinct days within 30 days | Recurrence across occasions marks a systematic interlanguage rule (§3.3). The 2-day rule stops one long text from promoting a slip. |
| `improving` | From `recurring`: over the last 3 texts with opportunities for p, pooled accuracy ≥ 75% (and ≥ 4 opportunities in total) | Shows a measurable shift. The 75% threshold is a design choice halfway between the noticing stage and mastery, not taken from a study. |
| `mastered` | From `improving`: accuracy ≥ 90% in each of ≥ 3 consecutive opportunity-texts, ≥ 6 total opportunities, ≥ 14 days since the last error for p | Brown's (1973) criterion of 90% suppliance in obligatory contexts across 3 successive samples, later used in L2 morpheme studies (Dulay & Burt 1974). The 14-day gap stands in for a delayed post-test (§3.1, §3.5). |
| `relapsed` | An error for p while `mastered` | Behaves like `recurring` (back on the watch list), but the UI frames it gently ("came back, it happens"). Avoids hiding regressions. |
| `dormant` | No opportunities and no errors for 60 days | Status unknown. Never auto-promote to mastered from silence. |

Patterns that are `new` and see no repeat within 30 days are archived (kept for history, not shown).

**Watch list.** At most 3 patterns, chosen in this order: (1) any `relapsed`, (2) the `recurring` pattern with the most `days_p` in the last 30 days, then the highest `impedes_meaning` share, (3) `improving` before `recurring`. The user can pin or unpin patterns (autonomy). Patterns of kind `orthography.spelling` with different words never merge into one pattern; only repeated misspelling of *the same word* recurs.

**Caveat.** The 90%/3-sample rule comes from L1 child acquisition research. Using it for adult L2 accuracy in free writing is a convention adopted by analogy, not an empirically calibrated threshold. Make the thresholds configurable and revisit them with real data.

### 4.4 Deterministic vs LLM

| Step | Who | Why |
|---|---|---|
| Error labeling (top, sub, impedes_meaning, pattern_hint) | LLM, during correction | Needs language understanding. Constrained by the enum (§3.9). |
| Opportunity counts for watched patterns | LLM, during correction | Needs parsing in context. Only ≤ 3 patterns per call keeps it cheap and focused. |
| Mapping a pattern_hint to a pattern_key | LLM during idle; result stored | Hints vary in wording. Once stored, the mapping is stable and auditable. |
| Word counts, all rates, windows, trend tests, MTLD | Deterministic | Reproducible. LLMs are unreliable at arithmetic. |
| Lifecycle transitions, watch list, reminder ladder | Deterministic | Must be explainable ("mastered because..."). |
| Uptake detection | Deterministic (lemma/string match), optionally LLM-confirmed | Cheap. False positives are low-cost. |
| Prose summary (≤ 3 sentences + 1 focus tip) | LLM, *from the finished stats JSON only* | Natural phrasing. It must quote only numbers present in the JSON (check by re-parsing). |
| Consistency audit (κ on re-labels) | Deterministic over LLM re-labels | QA, not shown to the user. |

### 4.5 What not to show

- **Total or cumulative mistake counts.** They grow with usage, punish writing more, and point attention at the self (Kluger & DeNisi 1996).
- **A single score or grade.** No validated weighting of error types exists (Rifkin & Roberts 1995; Bryant et al. 2023).
- **A CEFR level estimate.** No validated mapping from error rates in short, mixed texts.
- **Peer comparison or leaderboards.** Not perceived positively by all learners (Jivet et al. 2018), and irrelevant for a single user.
- **Loss-framed daily streaks.** A broken streak can cause people to abandon the activity (Silverman & Barasch 2022), and missed days don't hurt habit formation (Lally et al. 2010).
- **Per-text error rates for short texts**, and any rate on fewer than 500 words. Too noisy.
- **Lexical diversity across tones**, or as a headline. Genre and length confounds (§3.6).
- **Suggestions as errors**, in any count.
- **Typos as patterns.** One-off misspellings in Slack tone are slips. They count in the orthography rate but never reach the watch list unless the same word recurs.

---

## 5. Draft JSON shape for the stats output

A sketch only. Field names are suggestions for the Rust types.

```json
{
  "schema_version": 1,
  "generated_at": "2026-10-23T18:00:00Z",
  "languages": {
    "de": {
      "declared_cefr": "A2",
      "totals": { "texts": 42, "words": 6120, "window_words": 1500 },
      "accuracy": {
        "error_rate_per_100w": { "recent": 6.1, "prior": 7.8, "rate_ratio": 0.78, "p_value": 0.03, "trend": "improving" },
        "global_error_rate_per_100w": { "recent": 0.4, "prior": 0.7, "trend": "stable" },
        "by_tone": [
          { "tone": "slack", "words": 2100, "error_rate_per_100w": 5.2, "trend": "stable" },
          { "tone": "formal_email", "words": 900, "error_rate_per_100w": 8.0, "trend": "insufficient_data" }
        ]
      },
      "categories": [
        { "id": "grammar", "rate_per_100w": 4.0, "share": 0.66, "trend": "improving",
          "subcategories": [
            { "id": "grammar.case", "rate_per_100w": 1.6, "share": 0.26, "trend": "stable", "kind": "rule" },
            { "id": "grammar.word_order", "rate_per_100w": 0.9, "share": 0.15, "trend": "improving", "kind": "rule" }
          ] }
      ],
      "patterns": [
        { "key": "de:grammar.case:dative-after-mit", "label": "Dative after 'mit'",
          "state": "improving", "since": "2026-10-02",
          "texts_with_error": 5, "days_with_error": 4,
          "recent_accuracy": 0.8, "opportunities_recent": 10,
          "last_error_at": "2026-10-15", "impedes_meaning_share": 0.0,
          "examples": [ { "text_id": "t_381", "original": "mit der Mann", "corrected": "mit dem Mann" } ],
          "reminder": { "ladder_step": 3, "next_due": "2026-10-29" },
          "on_watch_list": true, "pinned": false }
      ],
      "watch_list": ["de:grammar.case:dative-after-mit", "de:grammar.word_order:verb-final-weil"],
      "mastered_recently": [ { "key": "de:grammar.verb_form:perfekt-sein-bewegung", "mastered_at": "2026-10-20" } ],
      "suggestions": {
        "offered_90d": 64, "uptake_90d": 9, "uptake_rate": 0.14,
        "by_type": { "naturalness.collocation": 30, "naturalness.phrase": 21, "variety.synonym": 13 },
        "recent_uptakes": [ { "phrase": "eine Entscheidung treffen", "first_suggested": "2026-09-12", "reused_at": "2026-10-18" } ]
      },
      "lexical_diversity": { "enabled": false, "mtld_monthly_median": [ { "month": "2026-09", "tone": "slack", "value": 58.2, "n_texts": 7 } ] }
    }
  },
  "summary": {
    "text": "Your German case endings with 'mit' are improving: 8 of your last 10 uses were correct. Word order after 'weil' is the next thing to watch.",
    "focus_tip": { "pattern_key": "de:grammar.word_order:verb-final-weil", "tip": "After 'weil', the conjugated verb goes to the end." },
    "numbers_verified": true
  },
  "quality": { "label_kappa_sub": 0.68, "label_kappa_top": 0.81, "audit_sample": 40 }
}
```

`trend` is one of `improving | stable | worsening | insufficient_data`. `state` is one of `new | recurring | improving | mastered | relapsed | dormant`.

---

## 6. Where the evidence is weak (honest list)

- **Transfer of WCF research to this setting.** The studies are mostly students in classrooms, focused on English articles. Refine's user is an adult professional writing freely in two languages.
- **Mastery thresholds.** The 90%/3-sample rule is borrowed from L1 acquisition research. The 75% "improving" threshold, the 14-day gap and the 500-word minimum window are reasoned design choices, not empirically calibrated.
- **Spacing applied to grammar accuracy in writing** is an extrapolation from recall research.
- **LLM error-type labeling reliability** against a fixed taxonomy has no direct peer-reviewed benchmark I could find. Hence the built-in κ audit.
- **Error-gravity weights** are unsupported, so none are used.
- **Streaks and gamification.** Mixed, small effects; consumer-research evidence on streak breaking; company claims unverified.
- **i+1** is a heuristic, not a tested law. LLM CEFR tagging of suggestions is unvalidated.
- **Lexical diversity** as an individual progress signal on short, mixed-genre texts is unreliable.

---

## 7. References

Legend: **[verified-url]** means I opened a page (publisher, DOI registry/Crossref record, OpenAlex abstract record, ACL Anthology PDF or repository) confirming the citation. "Content:" says what I checked the claims against: *abstract*, *full text*, or *secondary* (findings confirmed only through secondary summaries).

- Bitchener, J., & Knoch, U. (2009). The value of a focused approach to written corrective feedback. *ELT Journal, 63*(3), 204–211. https://doi.org/10.1093/elt/ccn043 (online first 2008) [verified-url] Content: abstract.
- Bitchener, J., & Knoch, U. (2010). Raising the linguistic accuracy level of advanced L2 writers with written corrective feedback. *Journal of Second Language Writing, 19*(4), 207–217. https://doi.org/10.1016/j.jslw.2010.10.002 [verified-url] Content: bibliographic + summary only.
- Bodily, R., & Verbert, K. (2017). Review of research on student-facing learning analytics dashboards and educational recommender systems. *IEEE Transactions on Learning Technologies, 10*(4), 405–418. https://doi.org/10.1109/TLT.2017.2740172 [verified-url] Content: abstract.
- Boers, F., & Lindstromberg, S. (2012). Experimental and intervention studies on formulaic sequences in a second language. *Annual Review of Applied Linguistics, 32*, 83–110. https://doi.org/10.1017/S0267190512000050 [verified-url] Content: abstract.
- Boyd, A., Hana, J., Nicolas, L., Meurers, D., Wisniewski, K., et al. (2014). The MERLIN corpus: Learner language and the CEFR. *Proceedings of LREC 2014*, 1281–1288. https://doi.org/10.63317/23nds6r6pxq7 [verified-url] Content: abstract.
- Brown, D., Liu, Q., & Norouzian, R. (2023). Effectiveness of written corrective feedback in developing L2 accuracy: A Bayesian meta-analysis. *Language Teaching Research* (online first). https://doi.org/10.1177/13621688221147374 [verified-url] Content: abstract. (The second author's given name differs between sources.)
- Brown, R. (1973). *A first language: The early stages.* Harvard University Press. https://doi.org/10.4159/harvard.9780674732469 [verified-url] Content: secondary (the 90%-in-3-samples criterion).
- Bryant, C., Felice, M., & Briscoe, T. (2017). Automatic annotation and evaluation of error types for grammatical error correction. *Proceedings of ACL 2017*, 793–805. https://doi.org/10.18653/v1/P17-1074 [verified-url] Content: full text (Table 2, Table 3).
- Bryant, C., & Ng, H. T. (2015). How far are we from fully automatic high quality grammatical error correction? *Proceedings of ACL-IJCNLP 2015*, 697–707. https://doi.org/10.3115/v1/P15-1068 [verified-url] Content: secondary (cited in Bryant et al. 2023).
- Bryant, C., Yuan, Z., Qorib, M. R., Cao, H., Ng, H. T., & Briscoe, T. (2023). Grammatical error correction: A survey of the state of the art. *Computational Linguistics*; preprint arXiv:2211.05166v4. https://arxiv.org/abs/2211.05166 [verified-url] Content: full text (preprint).
- Burt, M. K. (1975). Error analysis in the adult EFL classroom. *TESOL Quarterly, 9*(1), 53–63. https://doi.org/10.2307/3586012 [verified-url] Content: abstract. (Page range from memory, not confirmed.)
- Burt, M. K., & Kiparsky, C. (1972). *The gooficon: A repair manual for English.* Newbury House. Existence confirmed through reviews, e.g. https://doi.org/10.2307/3585665 [verified-url] Content: secondary.
- Capel, A. (2012). Completing the English Vocabulary Profile: C1 and C2 vocabulary. *English Profile Journal, 3*. https://doi.org/10.1017/S2041536212000013 [verified-url] Content: abstract.
- Cepeda, N. J., Pashler, H., Vul, E., Wixted, J. T., & Rohrer, D. (2006). Distributed practice in verbal recall tasks: A review and quantitative synthesis. *Psychological Bulletin, 132*(3), 354–380. https://doi.org/10.1037/0033-2909.132.3.354 [verified-url] Content: abstract.
- Cepeda, N. J., Vul, E., Rohrer, D., Wixted, J. T., & Pashler, H. (2008). Spacing effects in learning: A temporal ridgeline of optimal retention. *Psychological Science, 19*(11), 1095–1102. https://doi.org/10.1111/j.1467-9280.2008.02209.x [verified-url] Content: abstract.
- Corder, S. P. (1967). The significance of learner's errors. *IRAL, 5*(1–4), 161–170. https://doi.org/10.1515/iral.1967.5.1-4.161 [verified-url] Content: secondary.
- Dahlmeier, D., Ng, H. T., & Wu, S. M. (2013). Building a large annotated corpus of learner English: The NUS Corpus of Learner English. *Proceedings of the 8th Workshop on Innovative Use of NLP for Building Educational Applications*, 22–31. https://aclanthology.org/W13-1703/ [verified-url] Content: full text (Table 3 kappas).
- Dulay, H. C., & Burt, M. K. (1974). Natural sequences in child second language acquisition. *Language Learning, 24*(1), 37–53. https://doi.org/10.1111/j.1467-1770.1974.tb00234.x [verified-url] Content: bibliographic only (cited as an L2 use of obligatory-context scoring; the details of their criterion are not checked).
- Ekiert, M., & di Gennaro, K. (2021). Focused written corrective feedback and linguistic target mastery: Conceptual replication of Bitchener and Knoch (2010). *Language Teaching, 54*(1), 71–89. https://doi.org/10.1017/S0261444819000120 [verified-url] Content: abstract. (Online 2019, issue 2021.)
- Ellis, R. (2009). A typology of written corrective feedback types. *ELT Journal, 63*(2), 97–107. https://doi.org/10.1093/elt/ccn023 [verified-url] Content: abstract. (Background only.)
- Ferris, D. (1999). The case for grammar correction in L2 writing classes: A response to Truscott (1996). *Journal of Second Language Writing, 8*(1), 1–11. https://doi.org/10.1016/S1060-3743(99)80110-6 [verified-url] Content: secondary.
- Ferris, D. R. (2004). The "grammar correction" debate in L2 writing: Where are we, and where do we go from here? (and what do we do in the meantime...?). *Journal of Second Language Writing, 13*(1), 49–62. https://doi.org/10.1016/j.jslw.2004.04.005 [verified-url] Content: bibliographic only.
- Ferris, D. R. (2006). Does error feedback help student writers? New evidence on the short- and long-term effects of written error correction. In K. Hyland & F. Hyland (Eds.), *Feedback in second language writing* (pp. 81–104). Cambridge University Press. https://doi.org/10.1017/CBO9781139524742.007 [verified-url] Content: secondary (treatable/untreatable).
- Ferris, D. R. (2011). *Treatment of error in second language student writing* (2nd ed.). University of Michigan Press. https://doi.org/10.3998/mpub.2173290 [verified-url] Content: secondary.
- Gilardi, F., Alizadeh, M., & Kubli, M. (2023). ChatGPT outperforms crowd workers for text-annotation tasks. *PNAS, 120*(30), e2305016120. https://doi.org/10.1073/pnas.2305016120 [verified-url] Content: abstract.
- Glaboniat, M., Müller, M., Rusch, P., Schmitz, H., & Wertenschlag, L. (2005). *Profile deutsch* (with CD-ROM). Langenscheidt (ISBN 3-468-49410-6; a later Klett edition 2017). https://osd.at/en/overview-profile-deutsch/ [verified-url] Content: publisher/ÖSD description. (Edition years vary across catalogues.)
- Han, Z. (2004). *Fossilization in adult second language acquisition.* Multilingual Matters. https://doi.org/10.21832/9781853596889 [verified-url] Content: bibliographic only.
- Hattie, J., & Timperley, H. (2007). The power of feedback. *Review of Educational Research, 77*(1), 81–112. https://doi.org/10.3102/003465430298487 [verified-url] Content: abstract.
- Jarvis, S. (2013). Capturing the diversity in lexical diversity. *Language Learning, 63*(s1), 87–106. https://doi.org/10.1111/j.1467-9922.2012.00739.x [verified-url] Content: abstract.
- Jivet, I., Scheffel, M., Drachsler, H., & Specht, M. (2017). Awareness is not enough: Pitfalls of learning analytics dashboards in the educational practice. *EC-TEL 2017, LNCS 10474*, 82–96. https://doi.org/10.1007/978-3-319-66610-5_7 [verified-url] Content: secondary.
- Jivet, I., Scheffel, M., Specht, M., & Drachsler, H. (2018). License to evaluate: Preparing learning analytics dashboards for educational practice. *Proceedings of LAK '18*, 31–40. https://doi.org/10.1145/3170358.3170421 [verified-url] Content: abstract.
- Kang, E., & Han, Z. (2015). The efficacy of written corrective feedback in improving L2 written accuracy: A meta-analysis. *The Modern Language Journal, 99*(1), 1–18. https://doi.org/10.1111/modl.12189 [verified-url] Content: abstract.
- Karpicke, J. D., & Roediger, H. L. (2008). The critical importance of retrieval for learning. *Science, 319*(5865), 966–968. https://doi.org/10.1126/science.1152408 [verified-url] Content: abstract.
- Kluger, A. N., & DeNisi, A. (1996). The effects of feedback interventions on performance: A historical review, a meta-analysis, and a preliminary feedback intervention theory. *Psychological Bulletin, 119*(2), 254–284. https://doi.org/10.1037/0033-2909.119.2.254 [verified-url] Content: abstract.
- Koizumi, R., & In'nami, Y. (2012). Effects of text length on lexical diversity measures: Using short texts with less than 200 tokens. *System, 40*(4), 554–564. https://doi.org/10.1016/j.system.2012.10.012 [verified-url] Content: secondary/summary. (A duplicate, withdrawn version exists at 10.1016/j.system.2012.10.017; cite the 554–564 version.)
- Krashen, S. D. (1985). *The input hypothesis: Issues and implications.* Longman. Existence confirmed through reviews, e.g. https://doi.org/10.2307/414800 [verified-url] Content: secondary.
- Lally, P., van Jaarsveld, C. H. M., Potts, H. W. W., & Wardle, J. (2010). How are habits formed: Modelling habit formation in the real world. *European Journal of Social Psychology, 40*(6), 998–1009. https://doi.org/10.1002/ejsp.674 [verified-url] Content: abstract. (Online 2009.)
- Landis, J. R., & Koch, G. G. (1977). The measurement of observer agreement for categorical data. *Biometrics, 33*(1), 159–174. https://doi.org/10.2307/2529310 [verified-url] Content: bibliographic only.
- Lim, S. C., & Renandya, W. A. (2020). Efficacy of written corrective feedback in writing instruction: A meta-analysis. *TESL-EJ, 24*(3). https://tesl-ej.org/wordpress/issues/volume24/ej95/ej95a3/ [verified-url] Content: abstract.
- Matcha, W., Uzir, N. A., Gašević, D., & Pardo, A. (2020). A systematic review of empirical studies on learning analytics dashboards: A self-regulated learning perspective. *IEEE Transactions on Learning Technologies, 13*(2), 226–245. https://doi.org/10.1109/TLT.2019.2916802 [verified-url] Content: abstract.
- McCarthy, P. M., & Jarvis, S. (2010). MTLD, vocd-D, and HD-D: A validation study of sophisticated approaches to lexical diversity assessment. *Behavior Research Methods, 42*(2), 381–392. https://doi.org/10.3758/BRM.42.2.381 [verified-url] Content: secondary.
- Murre, J. M. J., & Dros, J. (2015). Replication and analysis of Ebbinghaus' forgetting curve. *PLOS ONE, 10*(7), e0120644. https://doi.org/10.1371/journal.pone.0120644 [verified-url] Content: abstract.
- Nesselhauf, N. (2003). The use of collocations by advanced learners of English and some implications for teaching. *Applied Linguistics, 24*(2), 223–242. https://doi.org/10.1093/applin/24.2.223 [verified-url] Content: abstract.
- Nesselhauf, N. (2005). *Collocations in a learner corpus.* John Benjamins. https://doi.org/10.1075/scl.14 [verified-url] Content: bibliographic only.
- O'Keeffe, A., & Mark, G. (2017). The English Grammar Profile of learner competence: Methodology and key findings. *International Journal of Corpus Linguistics, 22*(4), 457–489. https://doi.org/10.1075/ijcl.14086.oke [verified-url] Content: publisher abstract (via search). (Crossref shows a later date; the publisher lists the 2017 issue.)
- Open Spaced Repetition. ABC of FSRS (project wiki). https://github.com/open-spaced-repetition/awesome-fsrs/wiki/ABC-of-FSRS [verified-url] Content: full page. (Not peer-reviewed.)
- Reznicek, M., Lüdeling, A., & Hirschmann, H. (2013). Competing target hypotheses in the Falko corpus. In A. Díaz-Negrillo et al. (Eds.), *Automatic treatment and analysis of learner corpus data* (pp. 101–124). John Benjamins. https://doi.org/10.1075/scl.59.07rez [verified-url] Content: abstract. (Editors from memory, not confirmed.)
- Rifkin, B., & Roberts, F. D. (1995). Error gravity: A critical review of research design. *Language Learning, 45*(3), 511–537. https://doi.org/10.1111/j.1467-1770.1995.tb00450.x [verified-url] Content: abstract.
- Roediger, H. L., & Karpicke, J. D. (2006). Test-enhanced learning: Taking memory tests improves long-term retention. *Psychological Science, 17*(3), 249–255. https://doi.org/10.1111/j.1467-9280.2006.01693.x [verified-url] Content: bibliographic only.
- Ryan, R. M., & Deci, E. L. (2000). Self-determination theory and the facilitation of intrinsic motivation, social development, and well-being. *American Psychologist, 55*(1), 68–78. https://doi.org/10.1037/0003-066X.55.1.68 [verified-url] Content: abstract.
- Sailer, M., & Homner, L. (2020). The gamification of learning: A meta-analysis. *Educational Psychology Review, 32*(1), 77–112. https://doi.org/10.1007/s10648-019-09498-w [verified-url] Content: abstract.
- Schmidt, R. W. (1990). The role of consciousness in second language learning. *Applied Linguistics, 11*(2), 129–158. https://doi.org/10.1093/applin/11.2.129 [verified-url] Content: abstract.
- Selinker, L. (1972). Interlanguage. *IRAL, 10*(1–4), 209–231. https://doi.org/10.1515/iral.1972.10.1-4.209 [verified-url] Content: bibliographic only.
- Settles, B., & Meeder, B. (2016). A trainable spaced repetition model for language learning. *Proceedings of ACL 2016*, 1848–1858. https://doi.org/10.18653/v1/P16-1174 [verified-url] Content: abstract.
- Sheen, Y. (2007). The effect of focused written corrective feedback and language aptitude on ESL learners' acquisition of articles. *TESOL Quarterly, 41*(2), 255–283. https://doi.org/10.1002/j.1545-7249.2007.tb00059.x [verified-url] Content: abstract.
- Shintani, N., Ellis, R., & Suzuki, W. (2014). Effects of written feedback and revision on learners' accuracy in using two English grammatical structures. *Language Learning, 64*(1), 103–131. https://doi.org/10.1111/lang.12029 [verified-url] Content: abstract. (Online 2013.)
- Silverman, J., & Barasch, A. (2022). On or off track: How (broken) streaks affect consumer decisions. *Journal of Consumer Research, 49*(6), 1095–1117. https://doi.org/10.1093/jcr/ucac029 [verified-url] Content: secondary (university press summary).
- Song, Y., Krishna, K., Bhatt, R., Gimpel, K., & Iyyer, M. (2024). GEE! Grammar error explanation with large language models. *Findings of NAACL 2024*, 754–781. https://doi.org/10.18653/v1/2024.findings-naacl.49 [verified-url] Content: abstract.
- Štindlová, B., Rosen, A., Hana, J., & Škodová, S. (2011). CzeSL – an error tagged corpus of Czech as a second language. Paper presented at PALC 2011, Łódź. https://jakobson.korpus.cz/~rosen/public/2011-czesl-palc.pdf [verified-url] Content: full text (Table 2). (Proceedings publisher not confirmed.)
- Truscott, J. (1996). The case against grammar correction in L2 writing classes. *Language Learning, 46*(2), 327–369. https://doi.org/10.1111/j.1467-1770.1996.tb01238.x [verified-url] Content: secondary.
- Truscott, J. (2007). The effect of error correction on learners' ability to write accurately. *Journal of Second Language Writing, 16*(4), 255–272. https://doi.org/10.1016/j.jslw.2007.06.003 [verified-url] Content: secondary (Semantic Scholar summary).
- Van Beuningen, C. G., De Jong, N. H., & Kuiken, F. (2012). Evidence on the effectiveness of comprehensive error correction in second language writing. *Language Learning, 62*(1), 1–41. https://doi.org/10.1111/j.1467-9922.2011.00674.x [verified-url] Content: abstract.
- Vann, R. J., Meyer, D. E., & Lorenz, F. O. (1984). Error gravity: A study of faculty opinion of ESL errors. *TESOL Quarterly, 18*(3), 427–440. https://doi.org/10.2307/3586713 [verified-url] Content: abstract. (Page range from memory, not confirmed.)
- Wisniewski, K., Woldt, C., Schöne, K., Abel, A., Blaschitz, V., Štindlová, B., & Vodičková, K. (2014). *The MERLIN annotation scheme for the annotation of German, Italian, and Czech learner language.* https://lt.eurac.edu/merlin/docs/MERLIN-annotation-scheme.pdf [verified-url] Content: full text (tag inventory, intelligibility tags).
- Wray, A. (2002). *Formulaic language and the lexicon.* Cambridge University Press. https://doi.org/10.1017/CBO9780511519772 [verified-url] Content: bibliographic only.
- Ye, J., Su, J., & Cao, Y. (2022). A stochastic shortest path algorithm for optimizing spaced repetition scheduling. *Proceedings of KDD '22*, 4381–4390. https://doi.org/10.1145/3534678.3539081 [verified-url] Content: abstract.
