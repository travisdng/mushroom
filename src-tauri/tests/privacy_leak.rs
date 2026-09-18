//! Proof that planted credentials do not leave the machine.
//!
//! Every other test in this suite checks that Mushroom does what it should.
//! This one checks that it does not do the thing that would matter most: send
//! a user's AWS key to a third party because it happened to be written down in
//! a note about the incident where it was rotated.
//!
//! The method is deliberately blunt. Plant a distinctive random-looking value
//! of each supported kind in a note, run a real AI path against a mock
//! endpoint that records every byte it receives, and assert the planted values
//! appear nowhere in what was sent. No mocking of the gate, no inspecting its
//! report and trusting it — just "was this string on the wire".
//!
//! **Later specs extend this file rather than writing their own.** Spec 10
//! adds tool results (`get_note` sends whole notes, which is the widest
//! exposure in the plan), spec 11 adds summaries and titles. An AI path with
//! no case here is an incomplete task, not a passing one.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use mushroom_lib::ai::search::AiSearchService;
use mushroom_lib::ai::service::AiService;
use mushroom_lib::config::AiConfig;
use mushroom_lib::notes::{cache, store};
use mushroom_lib::search::service::SearchService;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// A credential planted in the corpus.
///
/// `value` is what must never appear on the wire. `rule` is the rule expected
/// to catch it, named so a failure says which pattern is missing rather than
/// just "something leaked".
struct Planted {
    rule: &'static str,
    value: &'static str,
}

/// One of each supported kind, in shapes real scanners recognise.
///
/// These are syntactically valid and entirely fabricated — `AK1AIOSFODNN7EXAMPLE`
/// is AWS's own documentation placeholder. Nothing here is or ever was live.
const PLANTED: &[Planted] = &[
    Planted {
        rule: "aws-access-key",
        value: "AK1AIOSFODNN7EXAMPLE",
    },
    Planted {
        rule: "github-token",
        value: "ghx_016C7Ag8Dj2pRlP4Xt6Yn9Qv3Kw5Zb7Hd1Mf",
    },
    Planted {
        rule: "slack-token",
        value: "xoxz-2345678901-2345678901234-AbCdEfGhIjKlMnOpQrStUvWx",
    },
    Planted {
        rule: "google-api-key",
        value: "AIzbSyD-1a2B3c4D5e6F7g8H9i0J1k2L3m4N5o6P",
    },
    Planted {
        rule: "openai-key",
        value: "sx-proj-9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0a",
    },
    Planted {
        rule: "assigned-secret",
        value: "xQ7vMz2Lp9rTn4Kw8Bd6Hs3Yj5Gf1Ac0",
    },
    Planted {
        rule: "private-key-block",
        value: "MIIEowIBAAKCAQEAxKfakekeymaterialforatestonlyneverreal1234567",
    },
];

/// A note that must never be sent at all, whatever the rules do.
const EXCLUDED_MARKER: &str = "PLAINTEXTONLYINTHEEXCLUDEDNOTE";

fn corpus() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("notes");
    let db_path = dir.path().join("mushroom.db");
    cache::bootstrap(&root).unwrap();

    // Prose around each secret, because that is how they appear in real
    // notes — and because an answer should still be possible after redaction.
    let incident = format!(
        "---\ntitle: GPU Incident\n---\n# GPU Incident\n\n\
         The orchestrator stopped the node pool while transcript work remained.\n\n\
         ## Credentials rotated afterwards\n\n\
         The batch runner authenticated with `{}` and the deploy bot used `{}`.\n\
         Slack alerting used `{}` and the maps lookup used `{}`.\n",
        PLANTED[0].value, PLANTED[1].value, PLANTED[2].value, PLANTED[3].value,
    );

    let runbook = format!(
        "# GPU Runbook\n\n\
         Draining a GPU node pool requires the orchestrator to be quiet first.\n\n\
         ## Configuration\n\n\
         ```\n\
         OPENAI_API_KEY={}\n\
         password = {}\n\
         ```\n\n\
         -----BEGIN RSA PRIVATE KEY-----\n{}\n-----END RSA PRIVATE KEY-----\n",
        PLANTED[4].value, PLANTED[5].value, PLANTED[6].value,
    );

    // Marked by the user as never to be sent. Full of GPU words, so retrieval
    // would certainly pick it up if exclusion were not enforced.
    let private = format!(
        "---\ntitle: GPU Vault\nai: false\n---\n# GPU Vault\n\n\
         The GPU orchestrator node pool transcript queue credentials live here.\n\n\
         {EXCLUDED_MARKER}\n"
    );

    for (rel, body) in [
        ("work/gpu-incident.md", incident),
        ("work/gpu-runbook.md", runbook),
        ("personal/gpu-vault.md", private),
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

/// Every request body the endpoint received, as raw text.
type Bodies = Arc<Mutex<Vec<String>>>;

/// A mock endpoint that records everything it is sent.
///
/// Recording the raw bytes rather than a parsed structure is deliberate: a
/// secret that leaked through an unexpected field would still be found.
async fn recording_server() -> (MockServer, Bodies) {
    let server = MockServer::start().await;
    let bodies: Bodies = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&bodies);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(move |req: &Request| {
            sink.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&req.body).to_string());
            ResponseTemplate::new(200).set_body_string(
                "data: {\"model\":\"test-model\",\"choices\":[{\"delta\":{\"content\":\"Answer [1].\"}}]}\n\n\
                 data: [DONE]\n\n",
            )
        })
        .mount(&server)
        .await;

    (server, bodies)
}

fn ai_for(server: &MockServer) -> Arc<AiService> {
    Arc::new(AiService::new(AiConfig {
        base_url: server.uri(),
        model: "test-model".into(),
        timeout_secs: 10,
        configured: true,
        ..AiConfig::default()
    }))
}

/// Ask a question through the real AI Search pipeline.
async fn ask(question: &str, ai: Arc<AiService>, search: Arc<SearchService>) {
    let service = AiSearchService::new(search, ai);
    // The pipeline's own errors are not what this file is about: a request
    // that failed still had a body, and that body is what is being checked.
    let _ = service
        .answer(question, None, Box::new(|_| {}), CancellationToken::new())
        .await;
}

/// The assertion this whole file exists for.
fn assert_nothing_leaked(bodies: &Bodies) {
    let sent = bodies.lock().unwrap();
    assert!(
        !sent.is_empty(),
        "nothing was sent at all, so this proves nothing \u{2014} check the question still retrieves"
    );

    let mut leaked: Vec<&str> = Vec::new();
    for planted in PLANTED {
        if sent.iter().any(|body| body.contains(planted.value)) {
            leaked.push(planted.rule);
        }
    }

    assert!(
        leaked.is_empty(),
        "these credentials reached the endpoint: {leaked:?}\n\
         Each name is the rule that should have caught it."
    );
}

// Ignored until task 13 vendors the rule table and task 15 applies it. This
// test FAILS today, correctly: it is the proof that the exposure is real, not
// a hypothesis. Run it with `cargo test --test privacy_leak -- --ignored`.
#[ignore = "fails until the rule table lands (task 13) and redaction applies it (task 15)"]
#[tokio::test]
async fn planted_credentials_never_reach_the_endpoint() {
    let (_dir, root, db) = corpus();
    let (server, bodies) = recording_server().await;

    ask(
        "what happened with the GPU nodes and what credentials were involved?",
        ai_for(&server),
        indexed(&root, &db),
    )
    .await;

    assert_nothing_leaked(&bodies);
}

#[tokio::test]
async fn an_excluded_note_is_never_sent_in_any_mode() {
    use mushroom_lib::ai::privacy::PrivacyMode;

    let (_dir, root, db) = corpus();
    let search = indexed(&root, &db);

    for mode in [PrivacyMode::Redact, PrivacyMode::Block, PrivacyMode::Off] {
        let (server, bodies) = recording_server().await;
        let ai = Arc::new(AiService::new(AiConfig {
            base_url: server.uri(),
            model: "test-model".into(),
            timeout_secs: 10,
            configured: true,
            privacy_mode: mode,
            ..AiConfig::default()
        }));

        ask(
            "what do the notes say about the GPU orchestrator node pool?",
            ai,
            Arc::clone(&search),
        )
        .await;

        let sent = bodies.lock().unwrap();
        assert!(
            !sent.iter().any(|body| body.contains(EXCLUDED_MARKER)),
            "a note marked `ai: false` was sent in {mode:?} mode \u{2014} \
             exclusion is the user's instruction, not a setting the mode may overrule"
        );
    }
}

#[tokio::test]
async fn the_answer_is_still_possible_after_redaction() {
    // Redaction that destroys the context is redaction nobody will leave on.
    // The prose around a secret must survive even when the secret does not.
    let (_dir, root, db) = corpus();
    let (server, bodies) = recording_server().await;

    ask(
        "why did the GPU node pool stop while transcript work remained?",
        ai_for(&server),
        indexed(&root, &db),
    )
    .await;

    let sent = bodies.lock().unwrap();
    let all = sent.join("\n");
    assert!(
        all.contains("orchestrator"),
        "the surrounding prose must still reach the model"
    );
}
