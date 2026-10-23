//! Generates `src/lib/fixtures/demo.json`: a realistic eight-week history (English + German)
//! with stats computed by the real engine. Used by the browser mock and UI tests.
//!
//!     cargo run -p refine-core --example demo_data

use chrono::{Duration, Utc};
use refine_core::insights::{Insights, LanguageInsight};
use refine_core::model::{
    Cefr, Entry, Mistake, ModelRef, PatternCheck, SCHEMA_VERSION, Suggestion, SuggestionKind, ToneRef, pattern_key,
};
use refine_core::settings::{ProviderKind, Settings};
use refine_core::stats::{PatternPrefs, compute_stats};
use refine_core::taxonomy::ErrorCategory as C;
use refine_core::text::{find_from, to_span, word_count, word_diff};
use refine_core::tone::builtin_tones;
use serde_json::json;

struct M(&'static str, &'static str, C, &'static str, &'static str, bool);
struct S(SuggestionKind, &'static str, &'static str, &'static str, Cefr);

struct T {
    lang: &'static str,
    tone: &'static str,
    input: &'static str,
    mistakes: Vec<M>,
    suggestions: Vec<S>,
    /// (pattern label, category, correct uses) when the pattern was watched.
    watch: Vec<(&'static str, C, u32)>,
}

fn entry(i: usize, t: &T, days_ago: i64, hour: i64) -> Entry {
    let tones = builtin_tones();
    let tone = tones.iter().find(|x| x.id == t.tone).unwrap();
    let mut fixed = t.input.to_string();
    let mut cursor = 0;
    let mut mistakes = Vec::new();
    for (j, m) in t.mistakes.iter().enumerate() {
        let range = find_from(t.input, m.0, cursor).unwrap_or_else(|| panic!("{} not in {}", m.0, t.input));
        cursor = range.1;
        fixed = fixed.replacen(m.0, m.1, 1);
        mistakes.push(Mistake {
            id: format!("m{}", j + 1),
            original: m.0.into(),
            correction: m.1.into(),
            category: m.2,
            impedes_meaning: m.5,
            explanation: m.4.into(),
            pattern: m.3.into(),
            pattern_key: pattern_key(t.lang, m.2, m.3),
            span: Some(to_span(t.input, range)),
        });
    }
    let suggestions = t
        .suggestions
        .iter()
        .enumerate()
        .map(|(j, s)| Suggestion {
            id: format!("s{}", j + 1),
            kind: s.0,
            original: s.1.into(),
            replacement: s.2.into(),
            explanation: s.3.into(),
            level: s.4,
            span: find_from(&fixed, s.1, 0).map(|r| to_span(&fixed, r)),
        })
        .collect();
    let watch = t
        .watch
        .iter()
        .map(|(label, cat, correct)| {
            let key = pattern_key(t.lang, *cat, label);
            let errors = mistakes.iter().filter(|m| m.pattern_key == key).count() as u32;
            PatternCheck { key, label: label.to_string(), correct: *correct, errors }
        })
        .collect();
    let at = Utc::now() - Duration::days(days_ago) - Duration::hours(hour);
    Entry {
        schema_version: SCHEMA_VERSION,
        id: format!("demo-{i:03}"),
        created_at: at,
        input: t.input.into(),
        language: t.lang.into(),
        level: if t.lang == "de" { Cefr::B1 } else { Cefr::B2 },
        tone: ToneRef { id: tone.id.clone(), name: tone.name.clone() },
        model: ModelRef { provider_id: "openrouter".into(), provider_kind: ProviderKind::OpenRouter, model: "anthropic/claude-sonnet-4.5".into() },
        diff: word_diff(t.input, &fixed),
        fixed_text: fixed,
        mistakes,
        suggestions,
        watch,
        word_count: word_count(t.input),
        duration_ms: 2100 + (i as u64 * 137) % 1800,
        warnings: vec![],
    }
}

const MIT: &str = "dative after mit";
const VERB_LAST: &str = "verb last in subordinate clauses";
const PAST: &str = "past simple for finished past";
const BY: &str = "by for deadlines";

fn texts() -> (Vec<T>, Vec<T>) {
    use SuggestionKind::*;
    let early = vec![
        T {
            lang: "de",
            tone: "slack",
            input: "Hallo zusammen, ich habe gestern mit der Kunde gesprochen und er hat gesagt, dass er braucht mehr Zeit für die Tests. Ich melde mich morgen nochmal.",
            mistakes: vec![
                M("mit der Kunde", "mit dem Kunden", C::Case, MIT, "After 'mit' always use the dative: 'dem Kunden', not 'der Kunde'.", false),
                M("dass er braucht mehr Zeit", "dass er mehr Zeit braucht", C::WordOrder, VERB_LAST, "After 'dass' the conjugated verb goes to the end of the clause.", false),
            ],
            suggestions: vec![S(Phrase, "Ich melde mich morgen nochmal.", "Ich melde mich morgen wieder.", "'Wieder' sounds a bit more natural for getting back to someone.", Cefr::B1)],
            watch: vec![],
        },
        T {
            lang: "de",
            tone: "slack",
            input: "Sorry, ich bin heute etwas später im Büro, weil ich muss noch zum Arzt gehen. Fangt gerne ohne mich mit dem Daily an, ich lese das Protokoll später.",
            mistakes: vec![M("weil ich muss noch zum Arzt gehen", "weil ich noch zum Arzt gehen muss", C::WordOrder, VERB_LAST, "'Weil' sends the conjugated verb ('muss') to the end.", false)],
            suggestions: vec![],
            watch: vec![],
        },
        T {
            lang: "de",
            tone: "slack",
            input: "Wir treffen uns um 15 Uhr mit dem ganzen Team und dann machen wir eine Entscheidung über das neue Feature. Bitte bringt eure Notizen von letzter Woche mit.",
            mistakes: vec![M("machen wir eine Entscheidung", "treffen wir eine Entscheidung", C::Collocation, "eine Entscheidung treffen", "In German you 'treffen' a decision, you don't 'machen' it.", false)],
            suggestions: vec![S(Synonym, "Notizen", "Unterlagen", "'Unterlagen' covers notes, documents and slides. Common at work.", Cefr::B2)],
            watch: vec![],
        },
        T {
            lang: "de",
            tone: "email",
            input: "Sehr geehrte Frau Müller, vielen Dank für Ihre schnelle Antwort. Kannst du mir bitte die Rechnung noch einmal als PDF schicken? Mit freundlichen Grüßen",
            mistakes: vec![M("Kannst du mir", "Können Sie mir", C::Address, "Sie in formal emails", "You started with 'Sie' (Ihre); keep it: 'Können Sie', not 'Kannst du'.", false)],
            suggestions: vec![S(Phrase, "noch einmal als PDF schicken", "erneut als PDF zusenden", "'Zusenden' and 'erneut' fit the formal tone of an email.", Cefr::B2)],
            watch: vec![],
        },
        T {
            lang: "de",
            tone: "friends",
            input: "Ich fahre morgen früh mit den Zug nach Berlin und bleibe bis Sonntag. Hast du Lust, am Samstag zusammen essen zu gehen? Ich kenne da ein gutes Restaurant.",
            mistakes: vec![M("mit den Zug", "mit dem Zug", C::Case, MIT, "'Mit' takes the dative: 'dem Zug', not 'den Zug'.", false)],
            suggestions: vec![],
            watch: vec![],
        },
        T {
            lang: "de",
            tone: "slack",
            input: "Das Meeting war gut, aber ich denke dass wir mehr Zeit für die Planung brauchen. Kannst du mir dein Hilfe bei der Präsentation geben? Danke dir!",
            mistakes: vec![
                M("ich denke dass", "ich denke, dass", C::Punctuation, "comma before dass", "German always puts a comma before 'dass'.", false),
                M("dein Hilfe", "deine Hilfe", C::Gender, "die Hilfe", "'Hilfe' is feminine (die Hilfe), so it's 'deine'.", false),
            ],
            suggestions: vec![S(Collocation, "Hilfe bei der Präsentation geben", "bei der Präsentation helfen", "Shorter and more natural: 'Kannst du mir bei der Präsentation helfen?'", Cefr::B1)],
            watch: vec![],
        },
        T {
            lang: "en",
            tone: "slack",
            input: "Yesterday I have sent the report to the client, but he didn't answer yet. I'll ping him again tomorrow morning if there is still no reply by then.",
            mistakes: vec![
                M("I have sent", "I sent", C::VerbTense, PAST, "With 'yesterday' (a finished time), use the past simple: 'I sent'.", false),
                M("didn't answer yet", "hasn't answered yet", C::VerbTense, "present perfect with yet", "'Yet' goes with the present perfect: 'hasn't answered yet'.", false),
            ],
            suggestions: vec![],
            watch: vec![],
        },
        T {
            lang: "en",
            tone: "github",
            input: "Can you please explain me how the deploy pipeline works for the staging environment? I tried to follow the README, but the second step fails with a timeout.",
            mistakes: vec![M("explain me", "explain to me", C::Preposition, "explain to someone", "You explain something 'to' someone: 'explain to me'.", false)],
            suggestions: vec![S(Phrase, "the second step fails with a timeout", "the second step times out", "'Times out' is the usual way to say it in dev talk.", Cefr::B2)],
            watch: vec![],
        },
        T {
            lang: "en",
            tone: "email",
            input: "Dear Mark, we need to make a final decision until Friday, otherwise we will miss the release window. I will send you the informations about both options tomorrow.",
            mistakes: vec![
                M("until Friday", "by Friday", C::Preposition, BY, "For deadlines use 'by': 'by Friday'. 'Until' means a state continues up to then.", true),
                M("informations", "information", C::NounNumber, "information is uncountable", "'Information' has no plural in English.", false),
            ],
            suggestions: vec![S(Synonym, "make a final decision", "reach a final decision", "'Reach a decision' sounds a bit more polished in emails.", Cefr::C1)],
            watch: vec![],
        },
        T {
            lang: "en",
            tone: "slack",
            input: "Last week we have fixed the login bug, so the error rate should go down now. Since two weeks I am working on the export feature and it's almost ready.",
            mistakes: vec![
                M("Last week we have fixed", "Last week we fixed", C::VerbTense, PAST, "'Last week' is finished time, so use the past simple: 'we fixed'.", false),
                M("Since two weeks I am working", "For two weeks I have been working", C::VerbTense, "present perfect continuous with for", "For something that started in the past and continues: 'I have been working', and 'for' with a length of time.", false),
            ],
            suggestions: vec![],
            watch: vec![],
        },
    ];
    let late = vec![
        T {
            lang: "de",
            tone: "slack",
            input: "Ich habe gerade mit dem Kunden telefoniert und er hat gesagt, dass alles passt. Wir können also wie geplant am Montag mit dem Rollout starten.",
            mistakes: vec![],
            suggestions: vec![S(Synonym, "wie geplant", "planmäßig", "'Planmäßig' is a compact, common alternative at work.", Cefr::B2)],
            watch: vec![(MIT, C::Case, 2), (VERB_LAST, C::WordOrder, 1)],
        },
        T {
            lang: "de",
            tone: "slack",
            input: "Kurzes Update: Wir müssen bald eine Entscheidung treffen, weil der Kunde nächste Woche Bescheid haben möchte. Ich schreibe heute Abend mit dem Team eine Zusammenfassung.",
            mistakes: vec![],
            suggestions: vec![],
            watch: vec![(MIT, C::Case, 1), (VERB_LAST, C::WordOrder, 1)],
        },
        T {
            lang: "de",
            tone: "slack",
            input: "Ich bin morgen nicht da, weil ich mit meiner Schwester nach Hamburg fahre. Bei Fragen schreibt mir einfach, ich schaue ab und zu aufs Handy.",
            mistakes: vec![],
            suggestions: vec![S(Phrase, "schaue ab und zu aufs Handy", "schaue zwischendurch aufs Handy", "'Zwischendurch' is the everyday word for 'now and then' here.", Cefr::B1)],
            watch: vec![(MIT, C::Case, 1), (VERB_LAST, C::WordOrder, 1)],
        },
        T {
            lang: "de",
            tone: "slack",
            input: "Danke für das Feedback! Ich glaube, dass wir die Seite mit den neuen Farben deutlich übersichtlicher machen. Kannst du dir den Entwurf bis Freitag anschauen?",
            mistakes: vec![],
            suggestions: vec![],
            watch: vec![(MIT, C::Case, 1), (VERB_LAST, C::WordOrder, 1)],
        },
        T {
            lang: "de",
            tone: "slack",
            input: "Ich habe mit der neuen Kollegin gesprochen, weil sie Fragen zum Projekt hatte. Sie fängt nächste Woche an und braucht noch einen Zugang zum Server.",
            mistakes: vec![],
            suggestions: vec![S(Collocation, "braucht noch einen Zugang zum Server", "braucht noch Zugriff auf den Server", "'Zugriff auf' is the usual pairing for systems and servers.", Cefr::B2)],
            watch: vec![(MIT, C::Case, 1), (VERB_LAST, C::WordOrder, 1)],
        },
        T {
            lang: "de",
            tone: "slack",
            input: "Heute Morgen hat der Build wieder nicht funktioniert, weil ein Test kaputt war. Ich kümmere mich gleich darum und sage euch Bescheid, wenn es läuft.",
            mistakes: vec![],
            suggestions: vec![],
            watch: vec![(VERB_LAST, C::WordOrder, 2)],
        },
        T {
            lang: "en",
            tone: "slack",
            input: "Quick update: I sent the slides to the client this morning and they replied within an hour. We need their final feedback by Thursday so we can ship on Monday.",
            mistakes: vec![],
            suggestions: vec![S(Collocation, "final feedback", "final sign-off", "'Sign-off' is the common term for final approval.", Cefr::B2)],
            watch: vec![(PAST, C::VerbTense, 1), (BY, C::Preposition, 1)],
        },
        T {
            lang: "en",
            tone: "github",
            input: "Thanks for the review! I fixed the race condition you pointed out and added a test for it. Could you take another look when you have a moment?",
            mistakes: vec![],
            suggestions: vec![],
            watch: vec![(PAST, C::VerbTense, 2), (BY, C::Preposition, 0)],
        },
        T {
            lang: "en",
            tone: "slack",
            input: "Yesterday we had a long call with the design team and agreed on the new layout. I have to make a decision about the colors by tomorrow, any opinions?",
            mistakes: vec![M("make a decision", "make a decision", C::Collocation, "make a decision", "", false)],
            suggestions: vec![],
            watch: vec![(PAST, C::VerbTense, 2), (BY, C::Preposition, 1)],
        },
        T {
            lang: "en",
            tone: "email",
            input: "Dear Anna, thank you for your quick reply. I have attached the updated contract, and I would appreciate it if you could sign it by the end of the week.",
            mistakes: vec![],
            suggestions: vec![S(Synonym, "I would appreciate it if you could", "I'd be grateful if you could", "A warm, still formal alternative.", Cefr::B2)],
            watch: vec![(BY, C::Preposition, 1)],
        },
        T {
            lang: "en",
            tone: "slack",
            input: "Last night I tried the new build and it crashed twice when I opened the settings. I'll file an issue with the logs, it happens every time on my machine.",
            mistakes: vec![],
            suggestions: vec![],
            watch: vec![(PAST, C::VerbTense, 2)],
        },
    ];
    // The "make a decision" placeholder above (same original and correction) is not a real
    // mistake; drop it so the data stays honest.
    let late = late
        .into_iter()
        .map(|mut t| {
            t.mistakes.retain(|m| m.0 != m.1);
            t
        })
        .collect();
    (early, late)
}

fn main() {
    let (early, late) = texts();
    let mut entries = Vec::new();
    let mut i = 0;
    // Weeks 8..4: mostly the texts with mistakes; weeks 3..0: mostly the clean ones.
    for days_ago in (0..56).rev() {
        if days_ago % 7 == 6 {
            continue; // one quiet day a week
        }
        let pool = if days_ago >= 28 { &early } else { &late };
        let other = if days_ago >= 28 { &late } else { &early };
        for k in 0..2 {
            let t = if (days_ago + k) % 9 == 0 { &other[(i * 3) % other.len()] } else { &pool[(i * 7 + k as usize) % pool.len()] };
            entries.push(entry(i, t, days_ago, 9 - 4 * k));
            i += 1;
        }
    }
    let settings = Settings::default();
    let stats = compute_stats(&entries, &settings, &PatternPrefs::default(), &[], *chrono::Local::now().offset(), Utc::now());
    let insights = Insights {
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now() - Duration::hours(5),
        entry_count: stats.entry_count,
        model: ModelRef { provider_id: "openrouter".into(), provider_kind: ProviderKind::OpenRouter, model: "anthropic/claude-sonnet-4.5".into() },
        languages: vec![
            LanguageInsight {
                language: "de".into(),
                summary: "Word order after weil and dass is now consistently right; that pattern is mastered. The dative after 'mit' is improving and was right in your recent texts. 'Eine Entscheidung treffen' still slips back to 'machen' now and then.".into(),
                focus_pattern_key: stats.languages.iter().find(|l| l.language == "de").and_then(|l| l.watch_list.first().cloned()),
                tip: "'Mit' always takes the dative: mit dem Kunden, mit der Kollegin, mit den Kollegen.".into(),
            },
            LanguageInsight {
                language: "en".into(),
                summary: "Finished times like 'yesterday' and 'last week' now come with the past simple; that one is mastered. 'Yet' with the present perfect is the next thing to watch.".into(),
                focus_pattern_key: stats.languages.iter().find(|l| l.language == "en").and_then(|l| l.watch_list.first().cloned()),
                tip: "'Yet' pairs with the present perfect: 'He hasn't answered yet.'".into(),
            },
        ],
        numbers_verified: true,
    };
    let out = json!({
        "settings": settings,
        "tones": builtin_tones(),
        "entries": entries,
        "stats": stats,
        "insights": insights,
    });
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/lib/fixtures/demo.json");
    std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    let categories = concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/lib/fixtures/categories.json");
    std::fs::write(categories, serde_json::to_string_pretty(&refine_core::taxonomy::catalog()).unwrap()).unwrap();
    for l in &stats.languages {
        println!(
            "{}: {} texts, rate {:?} → {:?} ({:?}), watch {:?}",
            l.language, l.totals.texts, l.error_rate.prior, l.error_rate.recent, l.error_rate.trend, l.watch_list
        );
        for p in &l.patterns {
            println!("   {:<40} {:?} acc={:?}", p.label, p.state, p.recent_accuracy);
        }
    }
}
