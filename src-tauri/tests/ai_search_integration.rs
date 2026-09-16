//! The whole question pipeline, against a real index and a mock provider.
//!
//! Each stage has its own unit tests. What this file checks is that they
//! compose: that a question actually retrieves the right notes, that those
//! notes reach the model as excerpts, and that the answer comes back with its
//! citations resolved to real files.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use mushroom_lib::ai::search::{AiDelta, AiSearchService};
use mushroom_lib::ai::service::AiService;
use mushroom_lib::config::AiConfig;
use mushroom_lib::notes::{cache, store};
use mushroom_lib::search::service::SearchService;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn corpus() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("notes");
    let db_path = dir.path().join("mushroom.db");
    cache::bootstrap(&root).unwrap();

    for (rel, body) in [
        (
            "work/autoqa.md",
            "---\ntitle: AI AutoQA / Batch Incident\n---\n# AI AutoQA\n\nThe orchestrator can stop the node pool when it sees no active queue messages, even though transcript-processing work may still remain.\n\n## Root cause\n\nA GPU failure during drain leaves the batch half finished.\n",
        ),
        (
            "work/gpu-infra.md",
            "# GPU Infrastructure\n\nGPU nodes drain on a schedule. Capacity planning assumes no GPU failure during the drain window.\n",
        ),
        (
            "ideas/rag.md",
            "# RAG ideas\n\nChunking, embeddings, hybrid retrieval. Nothing about hardware in this note at all.\n",
        ),
    ] {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    (dir, root, db_path)
}

fn indexed(root: &Path, db_path: &Path) -> Arc<SearchService> {
    let svc = SearchService::new();
    svc.open(db_path).unwrap();
    svc.reconcile(root, &store::scan(root).notes).unwrap();
    Arc::new(svc)
}

/// An AI service pointed at a mock endpoint, with no key.
fn ai_for(server: &MockServer) -> Arc<AiService> {
    Arc::new(AiService::new(AiConfig {
        base_url: server.uri(),
        model: "test-model".into(),
        timeout_secs: 10,
        max_context_tokens: 6000,
        ..AiConfig::default()
    }))
}

/// A streamed chat-completions response producing `text`.
fn sse_answer(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "data: {{\"model\":\"test-model\",\"choices\":[{{\"delta\":{{\"content\":\"{escaped}\"}}}}]}}\n\n\
         data: {{\"usage\":{{\"prompt_tokens\":120,\"completion_tokens\":20,\"total_tokens\":140}}}}\n\n\
         data: [DONE]\n\n"
    )
}

/// Collects every delta, and the prompt the provider actually received.
struct Captured {
    deltas: Arc<Mutex<Vec<AiDelta>>>,
}

impl Captured {
    fn new() -> Self {
        Self {
            deltas: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn sink(&self) -> Box<dyn FnMut(AiDelta) + Send> {
        let handle = Arc::clone(&self.deltas);
        Box::new(move |delta| handle.lock().unwrap().push(delta))
    }

    fn kinds(&self) -> Vec<String> {
        self.deltas
            .lock()
            .unwrap()
            .iter()
            .map(|d| match d {
                AiDelta::Retrieved { .. } => "retrieved",
                AiDelta::Started { .. } => "started",
                AiDelta::Text { .. } => "text",
                AiDelta::Done { .. } => "done",
                AiDelta::Failed { .. } => "failed",
            })
            .map(str::to_string)
            .collect()
    }

    fn streamed_text(&self) -> String {
        self.deltas
            .lock()
            .unwrap()
            .iter()
            .filter_map(|d| match d {
                AiDelta::Text { delta } => Some(delta.clone()),
                _ => None,
            })
            .collect()
    }
}

/// Mount a streaming chat route and record the request body it receives.
async fn mount_answer(server: &MockServer, answer: &str) -> Arc<Mutex<Option<String>>> {
    let seen: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let handle = Arc::clone(&seen);
    let body = sse_answer(answer);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(move |req: &Request| {
            *handle.lock().unwrap() = Some(String::from_utf8_lossy(&req.body).to_string());
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body.clone())
        })
        .mount(server)
        .await;

    seen
}

#[tokio::test]
async fn a_question_retrieves_the_right_notes_and_returns_a_grounded_answer() {
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    let prompt = mount_answer(
        &server,
        "The orchestrator stops the node pool while transcript work remains [1].",
    )
    .await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let captured = Captured::new();

    let answer = service
        .answer(
            "What did I write about the GPU nodes shutting down too early?",
            None,
            captured.sink(),
            CancellationToken::new(),
        )
        .await
        .expect("the pipeline should have produced an answer");

    // The answer resolved its citation to a real note.
    assert!(!answer.no_results);
    assert_eq!(answer.grounding.used.len(), 1, "{:?}", answer.grounding);
    assert!(
        answer.grounding.used[0].note_id.contains("autoqa")
            || answer.grounding.used[0].note_id.contains("gpu"),
        "cited {:?}",
        answer.grounding.used[0].note_id
    );
    assert!(answer.grounding.unmatched.is_empty());
    assert!(!answer.grounding.uncited);

    // The excerpts really did reach the model.
    let sent = prompt.lock().unwrap().clone().expect("no request captured");
    assert!(sent.contains("=== EXCERPT 1 ==="), "{sent}");
    assert!(sent.contains("node pool"), "the note text was not sent");
    // And the question went verbatim (R3.6).
    assert!(sent.contains("shutting down too early"), "{sent}");
}

#[tokio::test]
async fn deltas_arrive_in_the_documented_order() {
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    mount_answer(&server, "Answer [1].").await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let captured = Captured::new();

    service
        .answer(
            "GPU failure",
            None,
            captured.sink(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    let kinds = captured.kinds();
    assert_eq!(kinds.first().map(String::as_str), Some("retrieved"));
    assert_eq!(kinds.get(1).map(String::as_str), Some("started"));
    assert_eq!(kinds.last().map(String::as_str), Some("done"));
    assert!(kinds.contains(&"text".to_string()), "{kinds:?}");
}

#[tokio::test]
async fn the_streamed_text_matches_the_final_answer() {
    // If these ever disagree, the panel shows one thing while the answer
    // records another.
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    mount_answer(&server, "Drains early during the window [1].").await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let captured = Captured::new();

    let answer = service
        .answer("GPU drain", None, captured.sink(), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(captured.streamed_text(), answer.text);
}

#[tokio::test]
async fn retrieved_notes_are_reported_before_the_model_is_called() {
    // R1.4: the user sees which notes were found while the model thinks.
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    mount_answer(&server, "Answer [1].").await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let captured = Captured::new();

    service
        .answer(
            "GPU failure",
            None,
            captured.sink(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    let deltas = captured.deltas.lock().unwrap();
    match deltas.first() {
        Some(AiDelta::Retrieved { passages, terms }) => {
            assert!(!passages.is_empty(), "nothing was reported as retrieved");
            assert!(terms.contains(&"gpu".to_string()), "{terms:?}");
        }
        other => panic!("expected Retrieved first, got {other:?}"),
    }
}

#[tokio::test]
async fn a_question_with_no_matching_notes_never_calls_the_model() {
    // The failure this milestone exists to prevent: asking a model to answer
    // from nothing, and getting a fluent invention.
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    mount_answer(&server, "I would have made something up.").await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let captured = Captured::new();

    let answer = service
        .answer(
            "What did I write about submarine sandwich recipes?",
            None,
            captured.sink(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    assert!(answer.no_results);
    assert!(answer.text.to_lowercase().contains("could not find"));
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "no request should have been made at all"
    );
    assert!(
        !captured.kinds().contains(&"started".to_string()),
        "the model was never started, so no Started delta"
    );
}

#[tokio::test]
async fn folder_scope_limits_which_notes_are_considered() {
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    let prompt = mount_answer(&server, "Answer [1].").await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));

    service
        .answer(
            "GPU failure",
            Some("ideas".into()),
            Captured::new().sink(),
            CancellationToken::new(),
        )
        .await
        .ok();

    // `ideas` has nothing about GPUs, so either nothing was sent at all or
    // what was sent came only from that folder.
    let sent = prompt.lock().unwrap().clone();
    if let Some(sent) = sent {
        assert!(!sent.contains("node pool"), "a work note escaped the scope");
    }
}

#[tokio::test]
async fn an_answer_citing_nothing_comes_back_marked_unverified() {
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    mount_answer(&server, "The pool drains on a schedule.").await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));

    let answer = service
        .answer(
            "GPU failure",
            None,
            Captured::new().sink(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    assert!(
        answer.grounding.uncited,
        "no marker, so nothing is verified"
    );
    assert!(!answer.grounding.unused.is_empty(), "excerpts were sent");
}

#[tokio::test]
async fn an_invented_citation_is_reported_rather_than_silently_shown() {
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    mount_answer(&server, "As established in your notes [9].").await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));

    let answer = service
        .answer(
            "GPU failure",
            None,
            Captured::new().sink(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(answer.grounding.unmatched, vec![9]);
}

#[tokio::test]
async fn a_provider_failure_emits_failed_and_keeps_the_retrieved_notes() {
    // R7.2: the notes that were found stay useful even when the model does not
    // answer.
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let captured = Captured::new();

    let outcome = service
        .answer(
            "GPU failure",
            None,
            captured.sink(),
            CancellationToken::new(),
        )
        .await;

    assert!(outcome.is_err());
    let kinds = captured.kinds();
    assert_eq!(kinds.first().map(String::as_str), Some("retrieved"));
    assert!(kinds.contains(&"failed".to_string()), "{kinds:?}");
}

#[tokio::test]
async fn cancelling_stops_the_answer() {
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(30))
                .set_body_string(sse_answer("too late")),
        )
        .mount(&server)
        .await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let cancel = CancellationToken::new();

    let token = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        token.cancel();
    });

    let started = std::time::Instant::now();
    let outcome = service
        .answer("GPU failure", None, Captured::new().sink(), cancel)
        .await;

    assert!(outcome.is_err());
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "cancelling should not wait for the response"
    );
}

#[tokio::test]
async fn the_context_budget_is_respected_and_reported() {
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    let prompt = mount_answer(&server, "Answer [1].").await;

    // A budget big enough for one excerpt only.
    let ai = Arc::new(AiService::new(AiConfig {
        base_url: server.uri(),
        model: "test-model".into(),
        timeout_secs: 10,
        max_context_tokens: 60,
        ..AiConfig::default()
    }));

    let service = AiSearchService::new(indexed(&root, &db), ai);

    let answer = service
        .answer(
            "GPU failure drain",
            None,
            Captured::new().sink(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    let sent = prompt.lock().unwrap().clone().unwrap();
    assert!(sent.contains("=== EXCERPT 1 ==="));
    assert!(
        !sent.contains("=== EXCERPT 3 ==="),
        "the budget was not enforced"
    );
    assert!(
        answer.dropped > 0,
        "excerpts were dropped but the user was not told"
    );
}

// --- Non-streaming fallback (R6.3) -------------------------------------

/// A one-shot (non-SSE) chat-completions response.
fn whole_answer(text: &str) -> serde_json::Value {
    serde_json::json!({
        "model": "test-model",
        "choices": [{ "message": { "role": "assistant", "content": text } }],
        "usage": { "prompt_tokens": 100, "completion_tokens": 10, "total_tokens": 110 }
    })
}

#[tokio::test]
async fn a_service_that_refuses_streaming_falls_back_without_telling_the_user() {
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;

    // Reject the streamed request, accept the plain one — which is how an
    // endpoint without SSE support actually behaves.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(wiremock::matchers::body_string_contains("\"stream\":true"))
        .respond_with(ResponseTemplate::new(400))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(whole_answer("Drains early [1].")))
        .with_priority(2)
        .mount(&server)
        .await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let captured = Captured::new();

    let answer = service
        .answer(
            "GPU failure",
            None,
            captured.sink(),
            CancellationToken::new(),
        )
        .await
        .expect("the fallback should have rescued this");

    assert_eq!(answer.text, "Drains early [1].");
    assert_eq!(answer.grounding.used.len(), 1, "still grounded");
    assert_eq!(
        captured.streamed_text(),
        answer.text,
        "same shape as streaming"
    );
    assert_eq!(kinds_of(&captured).last().map(String::as_str), Some("done"));
}

#[tokio::test]
async fn streaming_can_be_turned_off_in_config() {
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;

    let seen: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let handle = Arc::clone(&seen);
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(move |req: &Request| {
            *handle.lock().unwrap() = Some(String::from_utf8_lossy(&req.body).to_string());
            ResponseTemplate::new(200).set_body_json(whole_answer("One shot [1]."))
        })
        .mount(&server)
        .await;

    let ai = Arc::new(AiService::new(AiConfig {
        base_url: server.uri(),
        model: "test-model".into(),
        timeout_secs: 10,
        stream: false,
        ..AiConfig::default()
    }));

    let service = AiSearchService::new(indexed(&root, &db), ai);
    let answer = service
        .answer(
            "GPU failure",
            None,
            Captured::new().sink(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(answer.text, "One shot [1].");
    let sent = seen.lock().unwrap().clone().unwrap();
    assert!(
        sent.contains("\"stream\":false"),
        "streaming was not actually disabled: {sent}"
    );
}

fn kinds_of(captured: &Captured) -> Vec<String> {
    captured.kinds()
}

#[tokio::test]
async fn stopping_is_not_reported_as_a_failure() {
    // Pressing Stop is the user getting what they asked for. A Failed delta
    // renders as an error strip, which reads as telling them off.
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(30))
                .set_body_string(sse_answer("never arrives")),
        )
        .mount(&server)
        .await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let captured = Captured::new();
    let cancel = CancellationToken::new();

    let token = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        token.cancel();
    });

    let outcome = service
        .answer("GPU failure", None, captured.sink(), cancel)
        .await;

    assert!(outcome.is_err(), "the call still reports it did not finish");
    assert!(
        !captured.kinds().contains(&"failed".to_string()),
        "a cancellation must not arrive as a failure: {:?}",
        captured.kinds()
    );
    // The notes it found are still worth showing.
    assert_eq!(
        captured.kinds().first().map(String::as_str),
        Some("retrieved")
    );
}

#[tokio::test]
async fn a_stream_that_breaks_part_way_is_not_retried_as_a_whole_answer() {
    // The fallback exists for a provider that refuses `stream: true` outright.
    // A connection that dies *after* text has arrived is a different thing:
    // re-asking discards the partial answer the user can already see, pays for
    // the whole thing twice, and hides a real network fault.
    let (_dir, root, db) = corpus();
    let server = MockServer::start().await;

    // A stream that stops mid-answer: deltas, then nothing, no [DONE].
    let truncated = "data: {\"choices\":[{\"delta\":{\"content\":\"half an ans\"}}]}\n\n\
                     data: {\"choices\":[{\"delta\":{\"cont";

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(wiremock::matchers::body_string_contains("\"stream\":true"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(truncated),
        )
        .with_priority(1)
        .mount(&server)
        .await;
    // If the fallback fires, this is what it would get — a different answer,
    // which makes the mistake visible rather than silently plausible.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(whole_answer("REFETCHED WHOLE ANSWER [1].")),
        )
        .with_priority(2)
        .mount(&server)
        .await;

    let service = AiSearchService::new(indexed(&root, &db), ai_for(&server));
    let captured = Captured::new();

    let answer = service
        .answer(
            "GPU failure",
            None,
            captured.sink(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(
        answer.text, "half an ans",
        "the partial must be kept, not replaced by a second request"
    );
    assert!(
        !answer.text.contains("REFETCHED"),
        "the whole answer was re-requested behind the user's back"
    );
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        1,
        "one request only: the stream worked, the network did not"
    );
}
