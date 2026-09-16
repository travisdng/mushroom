//! Question in, grounded answer out.
//!
//! Composes the pieces that were each built and tested alone: question →
//! search terms → retrieval → excerpt block → prompt → provider → citation
//! check. Nothing here does any of that work itself, which is why each stage
//! could be tested without a database or a model.

use std::sync::Arc;

use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::ai::citations::{self, Grounding};
use crate::ai::context::{self, Citation, Excerpt};
use crate::ai::error::AiError;
use crate::ai::provider::{ChatRequest, Message, Usage};
use crate::ai::question::{self, SearchTerms};
use crate::ai::service::AiService;
use crate::search::retriever::RetrievedPassage;
use crate::search::service::SearchService;

/// The system prompt, compiled in but kept as a file so it can be read,
/// diffed and edited as prose rather than as a Rust string literal.
const SYSTEM_PROMPT: &str = include_str!("prompts/ai_search.md");
const EXCERPT_PLACEHOLDER: &str = "{{EXCERPTS}}";

/// How many passages to retrieve for a question, and how many any one note
/// may contribute. More than the search panel wants, because the model reads
/// them rather than scanning a list.
const PASSAGE_LIMIT: usize = 12;
const PER_NOTE_CAP: usize = 3;

/// What the user is told when retrieval found nothing at all (R2.5).
const NOTHING_FOUND: &str =
    "I could not find anything in your notes about that.\n\nTry different words — \
     search looks for the words you used, so a note that says the same thing \
     differently will not match yet.";

/// Progress from a running question, in the order it happens.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum AiDelta {
    /// Sent before the model is called, so the user sees which notes were
    /// found while it is still thinking (R1.4).
    Retrieved {
        passages: Vec<RetrievedPassage>,
        terms: Vec<String>,
    },
    Started {
        model: String,
    },
    Text {
        delta: String,
    },
    Done {
        answer: Box<AiAnswer>,
    },
    Failed {
        error: crate::error::AppErrorDto,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAnswer {
    pub text: String,
    pub model: String,
    pub usage: Option<Usage>,
    pub grounding: Grounding,
    /// Excerpts left out to fit the context budget (R3.7).
    pub dropped: usize,
    /// One excerpt alone exceeded the budget and was sent whole.
    pub oversized: bool,
    /// The terms actually searched, shown so a poor result is explicable.
    pub terms: Vec<String>,
    /// True when retrieval found nothing and no model call was made.
    pub no_results: bool,
    /// The model was asked and said it could not answer from the excerpts.
    /// Not the same as an ungrounded answer — this is the honest outcome, and
    /// flagging it as unverified would train people to ignore that warning.
    pub declined: bool,
    pub estimated_prompt_tokens: u32,
    pub latency_ms: u64,
}

/// Where streamed text goes. `FnMut` so a caller can push into a Tauri
/// channel; `Send` so the pipeline can run off the UI thread.
pub type DeltaSink = Box<dyn FnMut(AiDelta) + Send>;

pub struct AiSearchService {
    search: Arc<SearchService>,
    ai: Arc<AiService>,
}

impl AiSearchService {
    pub fn new(search: Arc<SearchService>, ai: Arc<AiService>) -> Self {
        Self { search, ai }
    }

    /// Answer a question from the user's notes.
    pub async fn answer(
        &self,
        question: &str,
        folder: Option<String>,
        mut sink: DeltaSink,
        cancel: CancellationToken,
    ) -> Result<AiAnswer, AiError> {
        let started = std::time::Instant::now();
        let terms = question::to_search_terms(question);

        let retrieved = self.retrieve(&terms, folder).await?;
        sink(AiDelta::Retrieved {
            passages: retrieved.passages.clone(),
            terms: terms.terms.clone(),
        });

        // No passages means no grounds for an answer. Calling the model here
        // would be asking it to invent one, which is the whole failure this
        // milestone is built to avoid (R2.5).
        if retrieved.passages.is_empty() {
            tracing::info!(
                target: "ai",
                terms = %terms.query,
                "no passages retrieved; not calling the model"
            );
            let answer = AiAnswer {
                text: NOTHING_FOUND.to_string(),
                model: String::new(),
                usage: None,
                grounding: citations::check("", &[]),
                dropped: 0,
                oversized: false,
                terms: terms.terms,
                no_results: true,
                declined: true,
                estimated_prompt_tokens: 0,
                latency_ms: started.elapsed().as_millis() as u64,
            };
            sink(AiDelta::Done {
                answer: Box::new(answer.clone()),
            });
            return Ok(answer);
        }

        let config = self.ai.config();
        let built = context::build(&retrieved.excerpts, config.max_context_tokens, PER_NOTE_CAP);

        let request = ChatRequest {
            model: config.model.clone(),
            messages: vec![
                Message::system(SYSTEM_PROMPT.replace(EXCERPT_PLACEHOLDER, &built.block)),
                // Verbatim (R3.6): rewriting the user's question would change
                // what they asked.
                Message::user(question),
            ],
            temperature: Some(config.temperature),
            max_tokens: None,
        };

        sink(AiDelta::Started {
            model: config.model.clone(),
        });

        let response = if config.stream {
            let mut delivered = 0usize;
            match self
                .stream_answer(request.clone(), &mut sink, &cancel, &mut delivered)
                .await
            {
                Ok(response) => Ok(response),
                // A provider that does not implement streaming rejects the
                // request outright, before any text arrives. One silent retry
                // without streaming beats telling the user their endpoint is
                // broken when it simply differs.
                //
                // `delivered == 0` is the whole distinction. A failure *after*
                // text has arrived proves streaming works and the network
                // broke — re-asking would throw away the partial answer the
                // user can already see, pay for the whole thing twice, and
                // hide a real fault.
                Err(err) if delivered == 0 && rejects_streaming(&err) => {
                    tracing::info!(
                        target: "ai",
                        "the service refused a streamed request; retrying without streaming"
                    );
                    self.whole_answer(request, &mut sink, &cancel).await
                }
                Err(err) => Err(err),
            }
        } else {
            self.whole_answer(request, &mut sink, &cancel).await
        };

        let response = match response {
            Ok(response) => response,
            Err(err) => {
                // A cancellation is the user's own doing, not a failure to
                // report back at them. The panel already knows it stopped, and
                // keeps whatever text had arrived.
                if !matches!(err, AiError::Cancelled) {
                    sink(AiDelta::Failed {
                        error: (&err).into(),
                    });
                }
                return Err(err);
            }
        };

        let grounding = citations::check(&response.content, &built.citations);

        if grounding.uncited {
            tracing::warn!(target: "ai", "the model cited none of the excerpts");
        }
        if !grounding.unmatched.is_empty() {
            tracing::warn!(
                target: "ai",
                unmatched = ?grounding.unmatched,
                "the model cited excerpts that were never sent"
            );
        }
        // Logged side by side so the estimate can be corrected from evidence
        // rather than guessed at twice (R3.3).
        tracing::info!(
            target: "ai",
            estimated_prompt_tokens = built.estimated_tokens,
            actual_prompt_tokens = response.usage.map(|u| u.prompt_tokens).unwrap_or(0),
            excerpts = built.citations.len(),
            dropped = built.dropped,
            "answer complete"
        );

        let declined = grounding.uncited && looks_like_a_refusal(&response.content);

        let answer = AiAnswer {
            declined,
            text: response.content,
            model: response.model,
            usage: response.usage,
            grounding,
            dropped: built.dropped,
            oversized: built.oversized,
            terms: terms.terms,
            no_results: false,
            estimated_prompt_tokens: built.estimated_tokens,
            latency_ms: started.elapsed().as_millis() as u64,
        };

        sink(AiDelta::Done {
            answer: Box::new(answer.clone()),
        });
        Ok(answer)
    }

    /// Stream the answer, forwarding each delta as it arrives.
    ///
    /// `delivered` counts the pieces actually emitted, so the caller can tell
    /// "this service will not stream" from "the stream broke part-way".
    async fn stream_answer(
        &self,
        request: ChatRequest,
        sink: &mut DeltaSink,
        cancel: &CancellationToken,
        delivered: &mut usize,
    ) -> Result<crate::ai::provider::ChatResponse, AiError> {
        // The sink is borrowed here but must be moved into the streaming
        // closure, so the text comes back through a channel we own.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let stream = self.ai.stream_chat(
            request,
            Box::new(move |delta: &str| {
                // A closed receiver only means nobody is listening; the
                // request itself is cancelled separately.
                let _ = tx.send(delta.to_string());
            }),
            cancel.clone(),
        );
        tokio::pin!(stream);

        loop {
            tokio::select! {
                Some(delta) = rx.recv() => {
                    *delivered += 1;
                    sink(AiDelta::Text { delta });
                }
                result = &mut stream => {
                    // Drain anything produced since the last poll, so no
                    // trailing text is lost.
                    while let Ok(delta) = rx.try_recv() {
                        *delivered += 1;
                        sink(AiDelta::Text { delta });
                    }
                    break result;
                }
            }
        }
    }

    /// Ask for the whole answer at once, then emit it as a single delta.
    ///
    /// The result shape is identical to the streamed path, so nothing
    /// downstream — the panel included — needs to know which was used.
    async fn whole_answer(
        &self,
        request: ChatRequest,
        sink: &mut DeltaSink,
        cancel: &CancellationToken,
    ) -> Result<crate::ai::provider::ChatResponse, AiError> {
        let response = self.ai.chat(request, cancel.clone()).await?;
        if !response.content.is_empty() {
            sink(AiDelta::Text {
                delta: response.content.clone(),
            });
        }
        Ok(response)
    }

    /// Retrieve passages off the async runtime — SQLite is blocking.
    async fn retrieve(
        &self,
        terms: &SearchTerms,
        folder: Option<String>,
    ) -> Result<Retrieved, AiError> {
        let search = self.search.clone();
        let query = terms.query.clone();

        let found = tokio::task::spawn_blocking(move || {
            search.retrieve_passages(&query, folder, PASSAGE_LIMIT, PER_NOTE_CAP)
        })
        .await
        .map_err(|e| AiError::BadResponse {
            detail: format!("the search task failed: {e}"),
        })?
        .map_err(|e| AiError::BadResponse {
            detail: format!("your notes could not be searched: {e}"),
        })?;

        let excerpts = found
            .passages
            .iter()
            .map(|p| Excerpt {
                note_id: p.note_id.clone(),
                note_title: p.note_title.clone(),
                folder: p.folder.clone(),
                heading_path: p.heading_path.clone(),
                line_start: p.line_start,
                line_end: p.line_end,
                text: p.text.clone(),
                modified: found.modified.get(&p.note_id).copied(),
            })
            .collect();

        Ok(Retrieved {
            passages: found.passages,
            excerpts,
        })
    }
}

/// Whether an answer is the model declining, rather than an ungrounded claim.
///
/// A refusal cites nothing because there was nothing to cite, which is correct.
/// The prompt asks for a specific form of words, so this looks for it; anything
/// it does not recognise still gets the unverified header, which is the safe
/// direction to be wrong in.
fn looks_like_a_refusal(answer: &str) -> bool {
    const MAX_REFUSAL_CHARS: usize = 400;

    let text = answer.trim().to_lowercase();
    if text.is_empty() || text.chars().count() > MAX_REFUSAL_CHARS {
        return false;
    }

    [
        "could not find",
        "couldn't find",
        "cannot find",
        "can't find",
        "do not contain",
        "don't contain",
        "does not contain",
        "doesn't contain",
        "no information",
        "not in your notes",
        "nothing in your notes",
    ]
    .iter()
    .any(|phrase| text.contains(phrase))
}

/// Whether the failure looks like "this endpoint does not do streaming".
///
/// A 400 is what an OpenAI-compatible service returns when it cannot honour
/// `stream: true`; a body it cannot parse as SSE shows up as a bad response.
fn rejects_streaming(err: &AiError) -> bool {
    matches!(
        err,
        AiError::ServerError { status: 400 } | AiError::BadResponse { .. }
    )
}

struct Retrieved {
    passages: Vec<RetrievedPassage>,
    excerpts: Vec<Excerpt>,
}

/// The citation map for an answer, for callers that need to resolve a click
/// back to a note after the fact.
pub fn sources(grounding: &Grounding) -> Vec<Citation> {
    grounding.used.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_file_is_present_and_has_its_placeholder() {
        assert!(
            SYSTEM_PROMPT.contains(EXCERPT_PLACEHOLDER),
            "the excerpts would never reach the model"
        );
        assert!(SYSTEM_PROMPT.len() > 200, "the prompt looks truncated");
    }

    #[test]
    fn the_prompt_states_the_rules_that_keep_an_answer_grounded() {
        let lowered = SYSTEM_PROMPT.to_lowercase();
        // Each of these is a rule the milestone's honesty check depends on.
        for required in [
            "only",           // answer only from the excerpts
            "cite",           // cite what you used
            "could not find", // the honest non-answer
            "never invent",
        ] {
            assert!(
                lowered.contains(required),
                "the system prompt no longer says {required:?}"
            );
        }
    }

    #[test]
    fn substituting_excerpts_leaves_no_placeholder_behind() {
        let filled = SYSTEM_PROMPT.replace(EXCERPT_PLACEHOLDER, "=== EXCERPT 1 ===\nbody");
        assert!(!filled.contains(EXCERPT_PLACEHOLDER));
        assert!(filled.contains("=== EXCERPT 1 ==="));
    }

    #[test]
    fn a_refusal_is_recognised_so_it_is_not_flagged_as_unverified() {
        for answer in [
            "I could not find this in your notes.",
            "I could not find this information in your notes.",
            "Your notes do not contain anything about that.",
            "There is nothing in your notes about the cost.",
            "I cannot find an answer to that in the excerpts provided.",
        ] {
            assert!(looks_like_a_refusal(answer), "not recognised: {answer}");
        }
    }

    #[test]
    fn a_substantive_answer_is_not_mistaken_for_a_refusal() {
        for answer in [
            "The orchestrator stops the node pool while work remains.",
            "Capacity planning assumes no GPU failure during the drain window.",
            "",
        ] {
            assert!(
                !looks_like_a_refusal(answer),
                "wrongly treated as a refusal: {answer}"
            );
        }
    }

    #[test]
    fn a_long_answer_that_merely_mentions_not_finding_something_is_not_a_refusal() {
        // A real answer that happens to say "could not find" part-way through
        // still made claims, and those claims still need the header.
        let long = format!(
            "The drain window is scheduled nightly and the pool is resized then.              I could not find the exact times. {}",
            "The remaining detail is in the capacity note. ".repeat(12)
        );
        assert!(long.chars().count() > 400, "precondition");
        assert!(!looks_like_a_refusal(&long));
    }

    #[test]
    fn the_nothing_found_reply_admits_it_and_suggests_a_fix() {
        let lowered = NOTHING_FOUND.to_lowercase();
        assert!(lowered.contains("could not find"));
        assert!(
            lowered.contains("different words"),
            "a dead end with no suggestion is not a useful answer"
        );
    }
}
