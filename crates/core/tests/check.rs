use refine_core::check::{ModelCaps, run_check};
use refine_core::llm::{LlmClient, OutputMode, http_client};
use refine_core::model::{DiffOp, SuggestionKind};
use refine_core::prompt::{CheckContext, WatchedPattern, check_schema};
use refine_core::settings::{ProviderConfig, ProviderKind, Settings};
use refine_core::taxonomy::ErrorCategory;
use refine_core::tone::builtin_tones;
use serde_json::{Value, json};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn completion(content: &Value) -> ResponseTemplate {
    completion_text(&content.to_string())
}

fn completion_text(content: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "choices": [{ "message": { "role": "assistant", "content": content }, "finish_reason": "stop" }]
    }))
}

fn client(server: &MockServer, kind: ProviderKind) -> LlmClient {
    let provider = ProviderConfig {
        id: "test".into(),
        kind,
        label: "Test".into(),
        base_url: format!("{}/v1", server.uri()),
        model: "test-model".into(),
        needs_key: true,
        thinking: refine_core::settings::Thinking::Off,
    };
    LlmClient::new(http_client(), provider, Some("sk-test".into())).unwrap()
}

const INPUT: &str = "Yesterday I go to the office and we discussed about the deadline.";

fn good_answer() -> Value {
    json!({
        "notes": "S1: 'I go' with yesterday → past. 'discussed about' → no 'about'.",
        "language": "en",
        "mistakes": [
            { "original": "I go", "correction": "I went", "explanation": "Use the past tense for yesterday.", "category": "grammar.verb_tense", "pattern": "past simple for finished past", "impedesMeaning": false },
            { "original": "discussed about", "correction": "discussed", "explanation": "'Discuss' takes no 'about'.", "category": "grammar.preposition", "pattern": "discuss takes no about", "impedesMeaning": false }
        ],
        "fixedText": "Yesterday I went to the office and we discussed the deadline.",
        "review": "ok",
        "suggestions": [
            { "kind": "phrase", "original": "went to the office", "replacement": "went into the office", "explanation": "Common for going to work." }
        ]
    })
}

#[tokio::test]
async fn happy_path_builds_a_complete_entry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-test"))
        .and(body_partial_json(json!({ "model": "test-model", "response_format": { "type": "json_schema" } })))
        .respond_with(completion(&good_answer()))
        .expect(1)
        .mount(&server)
        .await;

    let settings = Settings::default();
    let tone = builtin_tones().into_iter().find(|t| t.id == "slack").unwrap();
    let ctx = CheckContext { settings: &settings, tone: &tone, language: None, known_patterns: &[], watched: &[] };
    let out = run_check(&client(&server, ProviderKind::OpenAiCompatible), &ctx, INPUT, ModelCaps::default()).await.unwrap();
    let e = out.entry;

    assert_eq!(out.caps, ModelCaps::default());
    assert_eq!(e.language, "en");
    assert_eq!(e.fixed_text, "Yesterday I went to the office and we discussed the deadline.");
    assert_eq!(e.mistakes.len(), 2);
    assert_eq!(e.mistakes[0].category, ErrorCategory::VerbTense);
    assert!(!e.mistakes[0].impedes_meaning);
    assert_eq!(e.mistakes[0].pattern_key, "en:grammar.verb_tense:past-simple-for-finished-past");
    assert_eq!(e.mistakes[0].span.unwrap().start, 10);
    assert_eq!(e.suggestions[0].kind, SuggestionKind::Phrase);
    assert!(e.diff.iter().any(|d| d.op == DiffOp::Delete && d.text.contains("go")));
    assert_eq!(e.tone.id, "slack");
    assert_eq!(e.word_count, 12);
    assert!(e.warnings.is_empty(), "{:?}", e.warnings);

    // What we got back conforms to the schema we sent.
    let sent: Value = serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
    let schema = &sent["response_format"]["json_schema"]["schema"];
    assert_eq!(schema, &check_schema(&ctx));
    assert!(jsonschema::is_valid(schema, &good_answer()));
}

#[tokio::test]
async fn bad_answer_gets_one_repair_round() {
    let server = MockServer::start().await;
    let mut bad = good_answer();
    bad["mistakes"][0]["original"] = json!("I goes"); // not in the input
    let good = good_answer();
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(move |req: &Request| {
            let body: Value = serde_json::from_slice(&req.body).unwrap();
            let n = body["messages"].as_array().unwrap().len();
            if n == 2 { completion(&bad) } else { completion(&good) }
        })
        .expect(2)
        .mount(&server)
        .await;

    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let ctx = CheckContext { settings: &settings, tone: &tone, language: Some("en"), known_patterns: &[], watched: &[] };
    let e = run_check(&client(&server, ProviderKind::OpenAiCompatible), &ctx, INPUT, ModelCaps::default()).await.unwrap().entry;
    assert_eq!(e.mistakes.len(), 2);
    assert!(e.warnings.is_empty());

    let reqs = server.received_requests().await.unwrap();
    let repair: Value = serde_json::from_slice(&reqs[1].body).unwrap();
    let last = repair["messages"].as_array().unwrap().last().unwrap()["content"].as_str().unwrap().to_string();
    assert!(last.contains("\"I goes\" is not copied verbatim"), "{last}");
}

#[tokio::test]
async fn falls_back_when_json_schema_is_unsupported() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "response_format": { "type": "json_schema" } })))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({ "error": { "message": "response_format json_schema is not supported by this model" } })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "response_format": { "type": "json_object" } })))
        .respond_with(completion_text(&format!("Here you go:\n```json\n{:#}\n```", good_answer())))
        .mount(&server)
        .await;

    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let ctx = CheckContext { settings: &settings, tone: &tone, language: None, known_patterns: &[], watched: &[] };
    let out = run_check(&client(&server, ProviderKind::OpenRouter), &ctx, INPUT, ModelCaps::default()).await;
    let out = out.unwrap();
    assert_eq!(out.caps.mode, OutputMode::JsonObject);
    assert_eq!(out.entry.mistakes.len(), 2);

    let reqs = server.received_requests().await.unwrap();
    let first: Value = serde_json::from_slice(&reqs[0].body).unwrap();
    assert_eq!(first["provider"]["require_parameters"], true);
    let second: Value = serde_json::from_slice(&reqs[1].body).unwrap();
    assert!(second["messages"][0]["content"].as_str().unwrap().contains("JSON Schema"));
}

#[tokio::test]
async fn silent_rewrite_without_mistakes_is_not_shown_as_a_fix() {
    let server = MockServer::start().await;
    let answer = json!({ "language": "en", "mistakes": [], "fixedText": "A totally different text.", "suggestions": [] });
    Mock::given(method("POST")).respond_with(completion(&answer)).expect(2).mount(&server).await;

    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let ctx = CheckContext { settings: &settings, tone: &tone, language: None, known_patterns: &[], watched: &[] };
    let e = run_check(&client(&server, ProviderKind::OpenAiCompatible), &ctx, INPUT, ModelCaps::default()).await.unwrap().entry;
    assert_eq!(e.fixed_text, INPUT);
    assert!(e.diff.iter().all(|d| d.op == DiffOp::Equal));
    assert_eq!(e.warnings.len(), 1);
}

#[tokio::test]
async fn provider_errors_are_readable() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({ "error": { "message": "No auth credentials found" } })))
        .mount(&server)
        .await;
    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let ctx = CheckContext { settings: &settings, tone: &tone, language: None, known_patterns: &[], watched: &[] };
    let err = run_check(&client(&server, ProviderKind::OpenRouter), &ctx, INPUT, ModelCaps::default()).await.err().unwrap();
    assert_eq!(err.to_string(), "Test answered with an error (401): No auth credentials found");
}

#[tokio::test]
async fn lists_openrouter_models_with_prices() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": [
            { "id": "b/model", "name": "B Model", "context_length": 128000,
              "pricing": { "prompt": "0.000003", "completion": "0.000015" },
              "supported_parameters": ["temperature", "structured_outputs"] },
            { "id": "a/model", "name": "A Model", "pricing": { "prompt": "0", "completion": "0" }, "supported_parameters": ["temperature"] }
        ]})))
        .mount(&server)
        .await;
    let models = client(&server, ProviderKind::OpenRouter).list_models().await.unwrap();
    assert_eq!(models[0].id, "a/model");
    assert_eq!(models[0].structured_outputs, Some(false));
    assert_eq!(models[1].prompt_price, Some(3.0));
    assert_eq!(models[1].completion_price, Some(15.0));
    assert_eq!(models[1].structured_outputs, Some(true));
}

#[tokio::test]
async fn watched_patterns_report_opportunities() {
    let server = MockServer::start().await;
    let mut answer = good_answer();
    answer["watch"] = json!([{ "pattern": "past simple for finished past", "correct": 2, "errors": 0 }]);
    Mock::given(method("POST")).respond_with(completion(&answer)).mount(&server).await;

    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let watched = vec![WatchedPattern {
        key: "en:grammar.verb_tense:past-simple-for-finished-past".into(),
        language: "en".into(),
        label: "past simple for finished past".into(),
    }];
    let ctx = CheckContext { settings: &settings, tone: &tone, language: None, known_patterns: &[], watched: &watched };
    let e = run_check(&client(&server, ProviderKind::OpenAiCompatible), &ctx, INPUT, ModelCaps::default()).await.unwrap().entry;
    assert_eq!(e.watch.len(), 1);
    assert_eq!(e.watch[0].correct, 2);
    // The model said 0 errors, but one mistake with this pattern is listed: never undercount.
    assert_eq!(e.watch[0].errors, 1);

    let sent: Value = serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
    assert!(sent["messages"][0]["content"].as_str().unwrap().contains("## Watched patterns"));
}

#[tokio::test]
async fn unknown_language_is_refused_not_misfiled() {
    let server = MockServer::start().await;
    let answer = json!({ "language": "other", "mistakes": [], "fixedText": "Bonjour à tous", "suggestions": [] });
    Mock::given(method("POST")).respond_with(completion(&answer)).expect(1).mount(&server).await;
    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let ctx = CheckContext { settings: &settings, tone: &tone, language: None, known_patterns: &[], watched: &[] };
    let err = run_check(&client(&server, ProviderKind::OpenAiCompatible), &ctx, "Bonjour à tous", ModelCaps::default())
        .await
        .err()
        .unwrap();
    assert!(err.to_string().contains("Add its language"), "{err}");
    let sent: Value = serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
    let langs = &sent["response_format"]["json_schema"]["schema"]["properties"]["language"]["enum"];
    assert_eq!(langs, &json!(["en", "de", "other"]));
}

#[tokio::test]
async fn mistakes_listed_out_of_order_still_map_to_the_right_place() {
    let server = MockServer::start().await;
    let input = "I go home and I go to bed.";
    let answer = json!({
        "language": "en",
        "mistakes": [
            { "original": "to bed", "correction": "to sleep", "explanation": "x", "category": "lexis.word_choice", "pattern": "p2", "impedesMeaning": false },
            { "original": "I go home", "correction": "I went home", "explanation": "x", "category": "grammar.verb_tense", "pattern": "p1", "impedesMeaning": false }
        ],
        "fixedText": "I went home and I go to sleep.",
        "suggestions": []
    });
    Mock::given(method("POST")).respond_with(completion(&answer)).mount(&server).await;
    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let ctx = CheckContext { settings: &settings, tone: &tone, language: Some("en"), known_patterns: &[], watched: &[] };
    let e = run_check(&client(&server, ProviderKind::OpenAiCompatible), &ctx, input, ModelCaps::default()).await.unwrap().entry;
    assert_eq!(e.mistakes[0].original, "I go home");
    assert_eq!(e.mistakes[0].span.unwrap().start, 0);
    assert_eq!(e.mistakes[1].span.unwrap().start, 19);
}

#[tokio::test]
async fn thinking_is_switched_off_and_falls_back_when_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "reasoning_effort": "none" })))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({ "error": { "message": "Unrecognized request argument: reasoning_effort" } })))
        .mount(&server)
        .await;
    Mock::given(method("POST")).respond_with(completion(&good_answer())).mount(&server).await;
    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let ctx = CheckContext { settings: &settings, tone: &tone, language: None, known_patterns: &[], watched: &[] };
    let out = run_check(&client(&server, ProviderKind::OpenAiCompatible), &ctx, INPUT, ModelCaps::default()).await.unwrap();
    assert!(!out.caps.no_thinking, "remembers that this model doesn't take the parameter");
    let reqs = server.received_requests().await.unwrap();
    let first: Value = serde_json::from_slice(&reqs[0].body).unwrap();
    let second: Value = serde_json::from_slice(&reqs[1].body).unwrap();
    assert_eq!(first["reasoning_effort"], "none");
    assert!(second.get("reasoning_effort").is_none());
}

#[tokio::test]
async fn unlisted_changes_never_reach_the_fixed_text() {
    let server = MockServer::start().await;
    let mut sneaky = good_answer();
    // Lists two fixes but also rewrites "the deadline" silently.
    sneaky["fixedText"] = json!("Yesterday I went to the office and we discussed the due date.");
    Mock::given(method("POST")).respond_with(completion(&sneaky)).expect(2).mount(&server).await;
    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let ctx = CheckContext { settings: &settings, tone: &tone, language: None, known_patterns: &[], watched: &[] };
    let e = run_check(&client(&server, ProviderKind::OpenAiCompatible), &ctx, INPUT, ModelCaps::default()).await.unwrap().entry;
    assert_eq!(e.fixed_text, "Yesterday I went to the office and we discussed the deadline.");
    assert_eq!(e.warnings.len(), 1, "{:?}", e.warnings);
    let reqs = server.received_requests().await.unwrap();
    let repair: Value = serde_json::from_slice(&reqs[1].body).unwrap();
    let last = repair["messages"].as_array().unwrap().last().unwrap()["content"].as_str().unwrap().to_string();
    assert!(last.contains("aren't listed") && last.contains("due date"), "{last}");
}

#[tokio::test]
async fn a_failed_self_review_triggers_a_repair() {
    let server = MockServer::start().await;
    let mut doubtful = good_answer();
    doubtful["review"] = json!("'went to the office' is fine, but 'deadline' should be plural");
    let good = good_answer();
    Mock::given(method("POST"))
        .respond_with(move |req: &Request| {
            let body: Value = serde_json::from_slice(&req.body).unwrap();
            if body["messages"].as_array().unwrap().len() == 2 { completion(&doubtful) } else { completion(&good) }
        })
        .expect(2)
        .mount(&server)
        .await;
    let settings = Settings::default();
    let tone = builtin_tones().remove(0);
    let ctx = CheckContext { settings: &settings, tone: &tone, language: None, known_patterns: &[], watched: &[] };
    let e = run_check(&client(&server, ProviderKind::OpenAiCompatible), &ctx, INPUT, ModelCaps::default()).await.unwrap().entry;
    assert!(e.warnings.is_empty(), "{:?}", e.warnings);
}
