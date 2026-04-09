use super::{claude_append_system_prompt, claude_system_prompt};

#[test]
fn prompt_mentions_reply_status_contract() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("reply(to, text, status)"));
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
    assert!(prompt.contains("use the relevant superpowers workflow"));
    assert!(prompt.contains("review delivered code"));
    assert!(prompt.contains("report verified results to the user"));
    assert!(prompt.contains("think deeply"));
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
fn lead_prompt_requires_communication_checklist() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("Current step"));
    assert!(prompt.contains("What was verified"));
    assert!(prompt.contains("Blocker / risk / external dependency"));
    assert!(prompt.contains("Next action and owner"));
    assert!(prompt.contains("Acceptance level"));
}

#[test]
fn lead_prompt_requires_acceptance_layer_distinction() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("stage_complete"));
    assert!(prompt.contains("final_acceptance"));
    assert!(prompt.contains(
        "MUST NOT present work as finally accepted or completed unless real-environment"
    ));
}

#[test]
fn lead_prompt_requires_blocker_disclosure_for_external_deps() {
    let prompt = claude_system_prompt("lead");
    assert!(prompt.contains("external dependency blocks true completion"));
    assert!(prompt.contains("explicitly surface that blocker"));
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
    assert!(prompt.contains("Web crawling"));
    assert!(prompt.contains("reverse engineering"));
    assert!(prompt.contains("social engineering scripts"));
}

#[test]
fn factual_error_correction_still_respects_routing_policy() {
    let prompt = claude_system_prompt("coder");
    assert!(prompt.contains("significant factual error"));
    assert!(prompt.contains("still follow the routing policy"));
}

#[test]
fn append_prompt_mentions_role_specific_handoff_contract() {
    let prompt = claude_append_system_prompt("coder");
    assert!(prompt.contains("role `coder`"));
    assert!(prompt.contains("default recipient is lead"));
}
