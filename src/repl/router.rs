use anyhow::Result;
use colored::Colorize;

use crate::cli::state::AppState;
use crate::executor::{AgentExecutor, ClaudeExecutor, ExecutionContext};
use crate::repl::session::Session;
use crate::types::Task;

/// Route natural-language input through Claude for intent classification,
/// then execute the appropriate internal command or return a conversation response.
pub async fn route_natural_language(
    input: &str,
    state: &AppState,
    session: &mut Session,
) -> Result<()> {
    session.push_user(input);

    let context_history = session.format_for_claude();

    let system_prompt = format!(
        "You are Panopticon's intent router. Given the user's message and conversation history, \
         classify the intent and respond with a JSON object.\n\n\
         Available actions:\n\
         - {{\"action\": \"plan\", \"goal\": \"<goal description>\"}}\n\
         - {{\"action\": \"execute\", \"all\": true}}\n\
         - {{\"action\": \"execute\", \"id\": \"<task-uuid>\"}}\n\
         - {{\"action\": \"status\"}}\n\
         - {{\"action\": \"task_list\"}}\n\
         - {{\"action\": \"agent_list\"}}\n\
         - {{\"action\": \"help\"}}\n\
         - {{\"action\": \"conversation\", \"response\": \"<your helpful response>\"}}\n\n\
         If the user is asking you to plan, decompose, or analyze something into tasks, use \"plan\".\n\
         If the user wants to run/execute tasks, use \"execute\".\n\
         If the user is asking about current state, use \"status\".\n\
         If none of the above match, use \"conversation\" and provide a helpful response.\n\n\
         {context_history}\n\
         Respond with ONLY the JSON object, no markdown, no explanation."
    );

    let task = Task::new(
        "intent_classification",
        format!("User message: {input}"),
    );

    let ctx = ExecutionContext {
        system_prompt: Some(system_prompt),
        ..Default::default()
    };

    let executor = ClaudeExecutor::default().with_model("haiku");

    println!("{}", "Thinking...".dimmed());

    match executor.health_check().await {
        Ok(true) => {}
        _ => {
            // Claude CLI not available — provide a helpful fallback.
            let msg = "Claude CLI is not available. Use slash commands instead (/help for list).";
            println!("{}", msg.yellow());
            session.push_assistant(msg);
            return Ok(());
        }
    }

    let result = executor.execute(&task, &ctx).await;

    match result {
        Ok(result) => {
            // Try to parse as a structured action.
            let action = result
                .output
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("conversation");

            match action {
                "plan" => {
                    let goal = result
                        .output
                        .get("goal")
                        .and_then(|v| v.as_str())
                        .unwrap_or(input);
                    session.push_assistant(&format!("Planning: {goal}"));
                    crate::cli::commands::plan::handle(goal, "sonnet", state).await?;
                }
                "execute" => {
                    let all = result
                        .output
                        .get("all")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let id: Option<uuid::Uuid> = result
                        .output
                        .get("id")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok());

                    match id {
                        Some(task_id) if !all => {
                            session
                                .push_assistant(&format!("Executing task {task_id}..."));
                            crate::cli::commands::execute::handle(
                                Some(task_id),
                                false,
                                "sonnet",
                                state,
                            )
                            .await?;
                        }
                        _ => {
                            session.push_assistant("Executing all pending tasks...");
                            crate::cli::commands::execute::handle(None, true, "sonnet", state)
                                .await?;
                        }
                    }
                }
                "status" => {
                    session.push_assistant("Showing status...");
                    crate::cli::commands::status::handle(state).await?;
                }
                "task_list" => {
                    session.push_assistant("Listing tasks...");
                    use crate::cli::TaskAction;
                    crate::cli::commands::task::handle(TaskAction::List, state).await?;
                }
                "agent_list" => {
                    session.push_assistant("Listing agents...");
                    use crate::cli::AgentAction;
                    crate::cli::commands::agent::handle(AgentAction::List, state).await?;
                }
                "help" => {
                    session.push_assistant("Showing help...");
                    crate::repl::output::print_help();
                }
                _ => {
                    let response = result
                        .output
                        .get("response")
                        .and_then(|v| v.as_str())
                        .unwrap_or("I'm not sure how to help with that. Try /help for available commands.");
                    println!("{}", response);
                    session.push_assistant(response);
                }
            }
        }
        Err(e) => {
            let msg = format!("Router error: {e}. Use slash commands instead (/help for list).");
            println!("{}", msg.yellow());
            session.push_assistant(&msg);
        }
    }

    Ok(())
}
