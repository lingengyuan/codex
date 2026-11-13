//! Verify that CLI predictions use the cache-preserving sample path.

use super::*;
use codex_protocol::ThreadId;
use pretty_assertions::assert_eq;
use uuid::Uuid;

#[test]
fn prompt_suggestion_samples_the_current_turn_with_low_effort() {
    let request = SuggestionRequest {
        thread_id: ThreadId::new(),
        turn_id: "turn-1".to_string(),
        id: Uuid::new_v4(),
        cancellation: tokio_util::sync::CancellationToken::new(),
    };

    let (request_id, params) = sample_request(&request);

    assert_eq!(
        request_id,
        RequestId::String(format!("prompt-suggestion-{}", request.id))
    );
    assert_eq!(params.thread_id, request.thread_id.to_string());
    assert_eq!(params.expected_turn_id, request.turn_id);
    assert_eq!(params.effort, Some(ReasoningEffort::Low));
    assert_eq!(
        params.thread_source,
        Some(ThreadSource::Feature("prompt_suggestion".to_string()))
    );
    assert!(params.disable_mcp);
    assert_eq!(params.interrupt_on_cancel, Some(true));
    assert!(params.prompt.ends_with(RESPONSE_CONTRACT));
}
