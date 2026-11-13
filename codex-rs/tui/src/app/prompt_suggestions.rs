//! Generate one hidden sample on the completed turn. `thread/sample` preserves the parent
//! context and request prefix, so prediction requests can reuse the prompt cache.

use super::*;
use crate::prompt_suggestions::SuggestionRequest;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadSampleCancelParams;
use codex_app_server_protocol::ThreadSampleCancelResponse;
use codex_app_server_protocol::ThreadSampleParams;
use codex_app_server_protocol::ThreadSampleResponse;
use codex_app_server_protocol::ThreadSource;
use codex_protocol::openai_models::ReasoningEffort;

const RESPONSE_CONTRACT: &str = r#"

Return only one JSON object with this exact shape:
{"suggestion":"one next user message in the user's voice, or null"}

Use null when every concrete next move would be arbitrary. Do not return Markdown or explanation.
"#;

impl App {
    pub(super) fn generate_prompt_suggestion(
        &mut self,
        app_server: &AppServerSession,
        request: SuggestionRequest,
    ) {
        if !self.local_settings.tui.prompt_suggestions
            || request.cancellation.is_cancelled()
            || self.chat_widget.thread_id() != Some(request.thread_id)
        {
            request.cancellation.cancel();
            return;
        }

        let handle = app_server.request_handle();
        let events = self.app_event_tx.clone();
        tokio::spawn(async move {
            let (sample_request_id, params) = sample_request(&request);

            let result = tokio::select! {
                biased;
                _ = request.cancellation.cancelled() => Err("suggestion cancelled".to_string()),
                result = handle.request_typed::<ThreadSampleResponse>(
                    ClientRequest::ThreadSample {
                        request_id: sample_request_id.clone(),
                        params,
                    },
                ) => result.map_err(|error| error.to_string()),
            };

            let text = if request.cancellation.is_cancelled() {
                let _ = tokio::time::timeout(
                    Duration::from_secs(/*secs*/ 3),
                    handle.request_typed::<ThreadSampleCancelResponse>(
                        ClientRequest::ThreadSampleCancel {
                            request_id: RequestId::String(format!(
                                "prompt-suggestion-cancel-{}",
                                request.id
                            )),
                            params: ThreadSampleCancelParams {
                                request_id: sample_request_id,
                            },
                        },
                    ),
                )
                .await;
                None
            } else {
                result
                    .ok()
                    .and_then(|response| response.text)
                    .and_then(|text| crate::prompt_suggestions::parse_suggestion(&text))
            };

            events.send(AppEvent::PromptSuggestionFinished { request, text });
        });
    }
}

fn sample_request(request: &SuggestionRequest) -> (RequestId, ThreadSampleParams) {
    (
        RequestId::String(format!("prompt-suggestion-{}", request.id)),
        ThreadSampleParams {
            thread_id: request.thread_id.to_string(),
            expected_turn_id: request.turn_id.clone(),
            prompt: format!("{}{}", crate::prompt_suggestions::PROMPT, RESPONSE_CONTRACT),
            effort: Some(ReasoningEffort::Low),
            thread_source: Some(ThreadSource::Feature("prompt_suggestion".to_string())),
            disable_mcp: true,
            interrupt_on_cancel: Some(true),
            ..Default::default()
        },
    )
}

#[cfg(test)]
#[path = "prompt_suggestions_tests.rs"]
mod tests;
