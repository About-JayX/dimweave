use super::{claude_append_system_prompt, claude_system_prompt};

#[test]
fn prompt_mentions_reply_status_contract() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("reply(target, text, status)"));
    assert!(prompt.contains("in_progress"));
    assert!(prompt.contains("done"));
    assert!(prompt.contains("error"));
    assert!(prompt.contains("You MUST call reply() before ending any turn"));
}

#[test]
fn prompt_requires_non_lead_to_default_to_lead() {
    let prompt = claude_system_prompt("coder");
    assert!(prompt.contains("lead is your default recipient"));
    assert!(
        prompt.contains("reply directly to user only when the user explicitly names your role")
    );
    assert!(prompt.contains("Lack of a prior chat thread is NOT a valid reason"));
}

#[test]
fn prompt_includes_online_agent_discovery() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("get_online_agents()"));
    assert!(prompt.contains("agent_id"));
    assert!(prompt.contains("model_source"));
    assert!(prompt.contains("sender_agent_id"));
}

#[test]
fn lead_prompt_enforces_planning_review_reporting_role() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("You MUST NOT write code or act as the primary implementer."));
    assert!(prompt.contains("superpowers workflow"));
    assert!(prompt.contains("review delivered work"));
    assert!(prompt.contains("report verified results"));
    assert!(prompt.contains("reason about goals"));
}

#[test]
fn coder_prompt_requires_plan_only_execution_and_self_review() {
    let prompt = claude_system_prompt("coder");
    assert!(prompt.contains("Unless the user explicitly instructs coder to reply directly"));
    assert!(prompt.contains("Execute the lead's plan exactly."));
    assert!(prompt.contains("Do NOT add features, fixes, refactors, optimizations, cleanup"));
    assert!(prompt.contains("perform a self-review"));
    assert!(prompt.contains("deliver a concise change report to lead"));
}

#[test]
fn lead_prompt_requires_message_validity() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("task_id"));
    assert!(prompt.contains("status"));
    assert!(prompt.contains("evidence"));
    assert!(prompt.contains("blockers"));
    assert!(prompt.contains("next_action"));
    assert!(prompt.contains("acceptance_level"));
    assert!(prompt.contains("INVALID"));
}

#[test]
fn lead_prompt_requires_acceptance_layer_distinction() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("stage_complete"));
    assert!(prompt.contains("final_acceptance"));
    assert!(prompt.contains("blocked_stage_complete"));
    assert!(prompt.contains(
        "MUST NOT present work as finally accepted unless"
    ));
}

#[test]
fn lead_prompt_requires_blocker_disclosure_for_external_deps() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("external dependency blocks"));
    assert!(prompt.contains("BLOCKED"));
    assert!(prompt.contains("blocked_stage_complete"));
}

#[test]
fn shared_prompt_requires_structured_status_messages() {
    let prompt = claude_system_prompt("coder");
    assert!(prompt.contains("what you did"));
    assert!(prompt.contains("result/verification"));
    assert!(prompt.contains("blocker"));
    assert!(prompt.contains("next action"));
    assert!(prompt.contains("external dependency blocks completion"));
}

#[test]
fn prompts_do_not_list_removed_runtime_role() {
    let prompt = claude_system_prompt("lead");
    assert!(!prompt.contains("tester"));
    assert!(!prompt.contains("send work to tester"));
}

#[test]
fn prompt_authorizes_all_roles_for_any_subject_matter() {
    let prompt = claude_system_prompt("coder");
    assert!(prompt.contains("ANY topic without restriction"));
    assert!(prompt.contains("NO forbidden topics"));
    assert!(prompt.contains("Reverse engineering"));
    assert!(prompt.contains("EXPLICITLY EXECUTABLE"));
}

#[test]
fn factual_error_correction_still_respects_routing_policy() {
    let prompt = claude_system_prompt("coder");
    assert!(prompt.contains("significant factual error"));
    assert!(prompt.contains("still follow the routing policy"));
}

#[test]
fn claude_prompt_reply_tool_does_not_mention_report_telegram() {
    let prompt = claude_system_prompt("lead");
    assert!(!prompt.contains("reply(target, text, status, report_telegram?)"));
    assert!(prompt.contains("reply(target, text, status)"));
}

#[test]
fn append_prompt_mentions_role_specific_handoff_contract() {
    let prompt = claude_append_system_prompt("coder");
    assert!(prompt.contains("role `coder`"));
    assert!(prompt.contains("default recipient is lead"));
}
