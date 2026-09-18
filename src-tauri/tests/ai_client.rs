//! The AI client against a local mock server.
//!
//! Every case an OpenAI-compatible endpoint can throw at us, with no test ever
//! contacting a real provider: no keys, no cost, no flakiness from someone
//! else's outage, and failures we can actually reproduce.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mushroom_lib::ai::client::{AiClient, StreamSink};
use mushroom_lib::ai::error::AiError;
use mushroom_lib::ai::privacy::{self, Policy, SanitisedRequest};
use mushroom_lib::ai::provider::{ChatRequest, Message, Provider};
use mushroom_lib::config::AiConfig;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn config_for(server: &MockServer) -> AiConfig {
    AiConfig {
        provider: Provider::LiteLlm,
        base_url: server.uri(),
        model: "test-model".into(),
        timeout_secs: 5,
        ..AiConfig::default()
    }
}

/// A request that has been through the privacy gate, because that is the only
/// kind the client accepts.
///
/// Note that this goes through `privacy::sanitise` like the real code does.
/// The sealed type has no test-only constructor on purpose: a back door here
/// would be a back door everywhere.
fn request() -> SanitisedRequest {
    privacy::sanitise(
        ChatRequest {
            model: "test-model".into(),
            messages: vec![Message::user("hello")],
            temperature: Some(0.2),
            max_tokens: Some(64),
        },
        &Policy::default(),
    )
    .expect("sanitising a plain request cannot fail")
}

fn completion_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-1",
        "model": "test-model-actual",
        "choices": [{ "index": 0, "message": { "role": "assistant", "content": content } }],
        "usage": { "prompt_tokens": 11, "completion_tokens": 7, "total_tokens": 18 }
    })
}

/// Mount a chat-completions route returning this template.
async fn mount_chat(server: &MockServer, template: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(template)
        .mount(server)
        .await;
}

async fn client_for(server: &MockServer) -> AiClient {
    AiClient::new(config_for(server), None).unwrap()
}

#[tokio::test]
async fn a_successful_completion_returns_content_and_usage() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_body("Hi there")),
    )
    .await;

    let response = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .expect("should have succeeded");

    assert_eq!(response.content, "Hi there");
    // The model the service reports wins over what we asked for: a proxy can
    // route elsewhere, and the user should see where the answer came from.
    assert_eq!(response.model, "test-model-actual");

    let usage = response.usage.expect("usage should have been parsed");
    assert_eq!(usage.prompt_tokens, 11);
    assert_eq!(usage.completion_tokens, 7);
    assert_eq!(usage.total_tokens, 18);
}

#[tokio::test]
async fn the_key_is_sent_as_a_bearer_token_when_there_is_one() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer sk-test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(completion_body("ok")))
        .mount(&server)
        .await;

    let client = AiClient::new(config_for(&server), Some("sk-test-key".into())).unwrap();
    assert!(client
        .chat(request(), CancellationToken::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn no_authorization_header_is_sent_when_there_is_no_key() {
    // A local LiteLLM proxy often wants no key at all, and sending an empty
    // bearer token is worse than sending none.
    let server = MockServer::start().await;
    let seen: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let flag = Arc::clone(&seen);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(move |req: &Request| {
            *flag.lock().unwrap() = req.headers.contains_key("authorization");
            ResponseTemplate::new(200).set_body_json(completion_body("ok"))
        })
        .mount(&server)
        .await;

    let client = AiClient::new(config_for(&server), None).unwrap();
    client
        .chat(request(), CancellationToken::new())
        .await
        .unwrap();
    assert!(
        !*seen.lock().unwrap(),
        "no key means no Authorization header"
    );
}

#[tokio::test]
async fn a_blank_key_counts_as_no_key() {
    let server = MockServer::start().await;
    let seen: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let flag = Arc::clone(&seen);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(move |req: &Request| {
            *flag.lock().unwrap() = req.headers.contains_key("authorization");
            ResponseTemplate::new(200).set_body_json(completion_body("ok"))
        })
        .mount(&server)
        .await;

    let client = AiClient::new(config_for(&server), Some("   ".into())).unwrap();
    client
        .chat(request(), CancellationToken::new())
        .await
        .unwrap();
    assert!(!*seen.lock().unwrap(), "whitespace is not a key");
}

#[tokio::test]
async fn a_401_is_reported_as_a_rejected_key() {
    let server = MockServer::start().await;
    mount_chat(&server, ResponseTemplate::new(401)).await;

    let err = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(matches!(err, AiError::Unauthorized), "{err:?}");
}

#[tokio::test]
async fn a_403_names_the_model() {
    let server = MockServer::start().await;
    mount_chat(&server, ResponseTemplate::new(403)).await;

    let err = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .unwrap_err();
    match err {
        AiError::Forbidden { model } => assert_eq!(model, "test-model"),
        other => panic!("expected Forbidden, got {other:?}"),
    }
}

#[tokio::test]
async fn a_404_carries_the_endpoint_so_the_v1_hint_can_be_built() {
    let server = MockServer::start().await;
    mount_chat(&server, ResponseTemplate::new(404)).await;

    let err = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .unwrap_err();
    match err {
        AiError::NotFound { endpoint, model } => {
            assert_eq!(endpoint, server.uri());
            assert_eq!(model, "test-model");
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn a_429_keeps_the_retry_after_value() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(429).insert_header("retry-after", "30"),
    )
    .await;

    let err = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .unwrap_err();
    match err {
        AiError::RateLimited { retry_after } => assert_eq!(retry_after, Some(30)),
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_json_is_a_bad_response_not_a_panic() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200).set_body_string("{ this is not json"),
    )
    .await;

    let err = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(matches!(err, AiError::BadResponse { .. }), "{err:?}");
}

#[tokio::test]
async fn a_200_with_no_choices_yields_empty_content_rather_than_an_error() {
    // Defensive: an empty answer is odd but not a failure, and crashing on it
    // would turn a shrug into a stack trace.
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200).set_body_json(serde_json::json!({ "choices": [] })),
    )
    .await;

    let response = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .unwrap();
    assert!(response.content.is_empty());
    assert_eq!(
        response.model, "test-model",
        "falls back to what we asked for"
    );
}

#[tokio::test]
async fn a_connection_refused_is_reported_as_a_connect_failure() {
    // Port 1 on loopback: nothing legitimate binds it, so this refuses
    // immediately and deterministically. Dropping a MockServer to free its
    // port does not work — the listener outlives the value long enough that
    // the connection still succeeds and returns a 404.
    let config = AiConfig {
        base_url: "http://127.0.0.1:1".into(),
        model: "test-model".into(),
        timeout_secs: 5,
        ..AiConfig::default()
    };
    let client = AiClient::new(config, None).unwrap();

    let err = client
        .chat(request(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(
        matches!(err, AiError::Connect { .. } | AiError::Dns { .. }),
        "{err:?}"
    );
}

#[tokio::test]
async fn a_slow_service_times_out_with_the_configured_number() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200)
            .set_delay(Duration::from_secs(30))
            .set_body_json(completion_body("too late")),
    )
    .await;

    let config = AiConfig {
        timeout_secs: 1,
        ..config_for(&server)
    };
    let client = AiClient::new(config, None).unwrap();

    let err = client
        .chat(request(), CancellationToken::new())
        .await
        .unwrap_err();
    match err {
        // The message quotes the user's own setting, so "no response after 1
        // second" matches what they typed in Settings.
        AiError::Timeout { seconds } => assert_eq!(seconds, 1),
        other => panic!("expected Timeout, got {other:?}"),
    }
}

// --- Retry (R6.5) ------------------------------------------------------

#[tokio::test]
async fn a_500_is_retried_once_and_then_succeeds() {
    let server = MockServer::start().await;

    // First call fails, second succeeds. Scoped mocks are matched in order of
    // specificity, so use up-to-n-times to sequence them.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(completion_body("second time")))
        .with_priority(2)
        .mount(&server)
        .await;

    let response = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .expect("the retry should have rescued this");
    assert_eq!(response.content, "second time");
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_500_twice_gives_up_rather_than_hammering() {
    let server = MockServer::start().await;
    mount_chat(&server, ResponseTemplate::new(500)).await;

    let err = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(
        matches!(err, AiError::ServerError { status: 500 }),
        "{err:?}"
    );
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        2,
        "one retry, not a storm"
    );
}

#[tokio::test]
async fn a_401_is_never_retried() {
    let server = MockServer::start().await;
    mount_chat(&server, ResponseTemplate::new(401)).await;

    let _ = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await;
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        1,
        "a bad key will still be bad the second time"
    );
}

#[tokio::test]
async fn a_429_is_never_retried() {
    // Retrying into a rate limit is the one thing guaranteed to make it worse.
    let server = MockServer::start().await;
    mount_chat(&server, ResponseTemplate::new(429)).await;

    let _ = client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

// --- Cancellation (R6.6) -----------------------------------------------

#[tokio::test]
async fn cancelling_stops_an_in_flight_request() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200)
            .set_delay(Duration::from_secs(30))
            .set_body_json(completion_body("never seen")),
    )
    .await;

    let client = client_for(&server).await;
    let cancel = CancellationToken::new();

    let token = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        token.cancel();
    });

    let started = std::time::Instant::now();
    let err = client.chat(request(), cancel).await.unwrap_err();

    assert!(matches!(err, AiError::Cancelled), "{err:?}");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "cancelling must not wait for the response: took {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_token_cancelled_before_the_call_never_sends_anything() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_body("x")),
    )
    .await;

    let cancel = CancellationToken::new();
    cancel.cancel();

    let err = client_for(&server)
        .await
        .chat(request(), cancel)
        .await
        .unwrap_err();
    assert!(matches!(err, AiError::Cancelled), "{err:?}");
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "an already-cancelled request must not reach the network"
    );
}

// --- Streaming (R6.7) --------------------------------------------------

fn sse(frames: &[&str]) -> String {
    let mut out = String::new();
    for f in frames {
        out.push_str(&format!(
            "data: {{\"model\":\"test-model-actual\",\"choices\":[{{\"delta\":{{\"content\":\"{f}\"}}}}]}}\n\n"
        ));
    }
    out.push_str("data: [DONE]\n\n");
    out
}

/// What the sink collected, shared with the test that reads it back.
type Collected = Arc<Mutex<Vec<String>>>;

/// Collect deltas as the client emits them.
fn sink() -> (StreamSink, Collected) {
    let collected: Collected = Arc::new(Mutex::new(Vec::new()));
    let handle = Arc::clone(&collected);
    let sink = Box::new(move |text: &str| {
        handle.lock().unwrap().push(text.to_string());
    });
    (sink, collected)
}

#[tokio::test]
async fn a_stream_arrives_in_pieces_and_assembles_correctly() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(sse(&["Once ", "upon ", "a time"])),
    )
    .await;

    let (sink, collected) = sink();
    let response = client_for(&server)
        .await
        .stream_chat(request(), sink, CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(response.content, "Once upon a time");
    assert_eq!(response.model, "test-model-actual");

    let pieces = collected.lock().unwrap().clone();
    assert_eq!(
        pieces.concat(),
        "Once upon a time",
        "the sink must see the same text the caller gets back"
    );
    assert!(
        pieces.len() > 1,
        "streaming should deliver several pieces, got {pieces:?}"
    );
}

#[tokio::test]
async fn a_stream_cut_mid_answer_keeps_what_arrived() {
    let server = MockServer::start().await;
    // No [DONE], and the last frame is cut in half.
    let truncated = "data: {\"choices\":[{\"delta\":{\"content\":\"half an ans\"}}]}\n\n\
                     data: {\"choices\":[{\"delta\":{\"cont";
    mount_chat(
        &server,
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(truncated),
    )
    .await;

    let (sink, collected) = sink();
    let response = client_for(&server)
        .await
        .stream_chat(request(), sink, CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(
        response.content, "half an ans",
        "a truncated stream keeps what did arrive"
    );
    assert_eq!(collected.lock().unwrap().concat(), "half an ans");
}

#[tokio::test]
async fn a_bad_frame_in_the_middle_does_not_lose_the_good_ones() {
    let server = MockServer::start().await;
    let body = "data: {\"choices\":[{\"delta\":{\"content\":\"good \"}}]}\n\n\
                data: {broken frame}\n\n\
                data: {\"choices\":[{\"delta\":{\"content\":\"still good\"}}]}\n\n\
                data: [DONE]\n\n";
    mount_chat(
        &server,
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(body),
    )
    .await;

    let (sink, _collected) = sink();
    let response = client_for(&server)
        .await
        .stream_chat(request(), sink, CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(response.content, "good still good");
}

#[tokio::test]
async fn a_stream_reports_usage_when_the_service_sends_it() {
    let server = MockServer::start().await;
    let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
                data: {\"choices\":[],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":1,\"total_tokens\":4}}\n\n\
                data: [DONE]\n\n";
    mount_chat(
        &server,
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(body),
    )
    .await;

    let (sink, _c) = sink();
    let response = client_for(&server)
        .await
        .stream_chat(request(), sink, CancellationToken::new())
        .await
        .unwrap();

    let usage = response.usage.expect("usage should have been picked up");
    assert_eq!(usage.total_tokens, 4);
}

#[tokio::test]
async fn a_stream_that_fails_before_it_starts_reports_the_http_error() {
    let server = MockServer::start().await;
    mount_chat(&server, ResponseTemplate::new(401)).await;

    let (sink, collected) = sink();
    let err = client_for(&server)
        .await
        .stream_chat(request(), sink, CancellationToken::new())
        .await
        .unwrap_err();

    assert!(matches!(err, AiError::Unauthorized), "{err:?}");
    assert!(
        collected.lock().unwrap().is_empty(),
        "nothing should have been emitted"
    );
}

#[tokio::test]
async fn streaming_asks_the_service_to_stream() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(sse(&["x"])),
    )
    .await;

    let (sink, _c) = sink();
    client_for(&server)
        .await
        .stream_chat(request(), sink, CancellationToken::new())
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["stream"], serde_json::json!(true));
    assert_eq!(body["model"], serde_json::json!("test-model"));
    assert_eq!(body["messages"][0]["role"], serde_json::json!("user"));
    assert_eq!(body["messages"][0]["content"], serde_json::json!("hello"));
}

#[tokio::test]
async fn a_non_streaming_request_does_not_ask_for_a_stream() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_body("x")),
    )
    .await;

    client_for(&server)
        .await
        .chat(request(), CancellationToken::new())
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["stream"], serde_json::json!(false));
    assert_eq!(body["temperature"], serde_json::json!(0.2));
    assert_eq!(body["max_tokens"], serde_json::json!(64));
}

// --- Test Connection (R5.1, R5.2) --------------------------------------

#[tokio::test]
async fn test_connection_reports_endpoint_model_and_latency() {
    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_body("pong")),
    )
    .await;

    let info = client_for(&server).await.test_connection().await.unwrap();

    assert_eq!(info.endpoint, server.uri());
    assert_eq!(info.model, "test-model");
    assert_eq!(
        info.provider_reported_model.as_deref(),
        Some("test-model-actual")
    );

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        body["max_tokens"],
        serde_json::json!(1),
        "a connectivity check should not generate a paragraph"
    );
}

#[tokio::test]
async fn test_connection_surfaces_a_401_plainly() {
    let server = MockServer::start().await;
    mount_chat(&server, ResponseTemplate::new(401)).await;

    let err = client_for(&server)
        .await
        .test_connection()
        .await
        .unwrap_err();
    assert!(matches!(err, AiError::Unauthorized), "{err:?}");
}

#[tokio::test]
async fn test_connection_refuses_to_guess_when_nothing_is_configured() {
    let config = AiConfig {
        base_url: String::new(),
        ..AiConfig::default()
    };
    let client = AiClient::new(config, None).unwrap();
    let err = client.test_connection().await.unwrap_err();
    assert!(matches!(err, AiError::Unconfigured), "{err:?}");
}

// --- Model listing (R4.1, R4.2) ----------------------------------------

#[tokio::test]
async fn models_are_listed_sorted_and_deduplicated() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "data": [
                { "id": "gpt-4.1-mini" },
                { "id": "claude-sonnet-4" },
                { "id": "gpt-4.1-mini" }
            ]
        })))
        .mount(&server)
        .await;

    let models = client_for(&server).await.list_models().await.unwrap();
    assert_eq!(models, vec!["claude-sonnet-4", "gpt-4.1-mini"]);
}

#[tokio::test]
async fn a_model_entry_without_an_id_is_skipped_not_fatal() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "usable" }, { "object": "model" }]
        })))
        .mount(&server)
        .await;

    let models = client_for(&server).await.list_models().await.unwrap();
    assert_eq!(models, vec!["usable"]);
}

#[tokio::test]
async fn an_endpoint_without_a_models_route_is_an_error_the_caller_can_soften() {
    // Not every OpenAI-compatible service implements /models. The client
    // reports it; the UI turns it into "unavailable" and a free-text field.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let err = client_for(&server).await.list_models().await.unwrap_err();
    assert!(matches!(err, AiError::NotFound { .. }), "{err:?}");
}

#[tokio::test]
async fn a_garbled_model_list_is_a_bad_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>nope</html>"))
        .mount(&server)
        .await;

    let err = client_for(&server).await.list_models().await.unwrap_err();
    assert!(matches!(err, AiError::BadResponse { .. }), "{err:?}");
}

// --- Containment (R6.3, R7.4) ------------------------------------------

/// The design says all HTTP to an AI service lives in `ai/`. That is only
/// true while nobody adds a `reqwest` call somewhere convenient, so check it
/// rather than trusting a doc comment.
#[test]
fn nothing_outside_ai_talks_http() {
    let mut offenders = Vec::new();

    for entry in walkdir::WalkDir::new("src")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
    {
        let path = entry.path();
        // Normalise so the check works on either path separator.
        let rel = path.to_string_lossy().replace('\\', "/");
        if rel.starts_with("src/ai/") {
            continue;
        }

        let text = std::fs::read_to_string(path).unwrap();
        if text.contains("reqwest") {
            offenders.push(rel);
        }
    }

    assert!(
        offenders.is_empty(),
        "HTTP belongs in src/ai/ only; found reqwest in: {offenders:?}"
    );
}

/// `test_connection` is exempt from the `SanitisedRequest` seal, because it
/// sends a fixed literal rather than anything the user wrote. This pins that
/// claim: if it ever starts echoing note content, a token in its signature
/// would be ceremony, but this fails the build.
#[tokio::test]
async fn test_connection_sends_nothing_but_its_own_ping() {
    let server = MockServer::start().await;
    let body: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let captured = Arc::clone(&body);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(move |req: &Request| {
            *captured.lock().unwrap() = serde_json::from_slice(&req.body).ok();
            ResponseTemplate::new(200).set_body_json(completion_body("pong"))
        })
        .mount(&server)
        .await;

    let config = AiConfig {
        configured: true,
        ..config_for(&server)
    };
    AiClient::new(config, None)
        .unwrap()
        .test_connection()
        .await
        .unwrap();

    let sent = body.lock().unwrap().clone().expect("a body was sent");
    let messages = sent["messages"].as_array().expect("messages is an array");

    assert_eq!(messages.len(), 1, "one message, not a conversation: {sent}");
    assert_eq!(
        messages[0]["content"].as_str(),
        Some("ping"),
        "test_connection must send its own literal and nothing else: {sent}"
    );
    // A connectivity check has no business asking for a completion either.
    assert_eq!(sent["max_tokens"].as_u64(), Some(1), "{sent}");
}

/// Fail closed: when the gate cannot run, nothing goes on the wire.
///
/// The unit tests prove `sanitise` refuses. This proves the refusal happens
/// *before* the HTTP call rather than after it — which is the half that
/// actually protects anything, and the half a reordered line could break
/// without any unit test noticing.
#[tokio::test]
async fn a_broken_privacy_gate_sends_nothing_at_all() {
    use mushroom_lib::ai::privacy::Rules;
    use mushroom_lib::ai::service::AiService;

    let server = MockServer::start().await;
    mount_chat(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_body("leaked")),
    )
    .await;

    let service = AiService::with_rules(
        AiConfig {
            configured: true,
            ..config_for(&server)
        },
        Rules::broken("the rule table did not load"),
    );

    let err = service
        .chat(
            ChatRequest {
                model: "test-model".into(),
                messages: vec![Message::user("the password is hunter2")],
                temperature: None,
                max_tokens: None,
            },
            CancellationToken::new(),
        )
        .await
        .unwrap_err();

    assert!(
        matches!(err, AiError::Privacy { .. }),
        "expected a privacy failure, got {err:?}"
    );
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "the gate failed, so the endpoint must have seen nothing at all"
    );
}
