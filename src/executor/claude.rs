use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::{AgentExecutor, ExecutionContext};
use crate::types::error::PanopticonError;
use crate::types::Task;
use crate::verification::TaskResult;

/// Executor that delegates tasks to the `claude` CLI as a subprocess.
#[derive(Debug, Clone)]
pub struct ClaudeExecutor {
    /// Model to use: "sonnet", "opus", "haiku"
    pub model: String,
    /// Permission mode: "bypassPermissions", "acceptEdits", "plan", etc.
    pub permission_mode: String,
    /// Max agentic turns.
    pub max_turns: Option<u32>,
    /// Allowed tools list (empty = all).
    pub allowed_tools: Vec<String>,
}

impl Default for ClaudeExecutor {
    fn default() -> Self {
        Self {
            model: "sonnet".to_string(),
            permission_mode: "bypassPermissions".to_string(),
            max_turns: Some(10),
            allowed_tools: Vec::new(),
        }
    }
}

impl ClaudeExecutor {
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Build the prompt from a task.
    fn build_prompt(task: &Task, context: &ExecutionContext) -> String {
        let mut prompt = String::new();

        if let Some(sys) = &context.system_prompt {
            prompt.push_str(sys);
            prompt.push_str("\n\n");
        }

        prompt.push_str(&format!("# Task: {}\n\n", task.name));
        prompt.push_str(&format!("{}\n\n", task.description));

        if !task.required_capabilities.is_empty() {
            prompt.push_str(&format!(
                "Required capabilities: {}\n\n",
                task.required_capabilities.join(", ")
            ));
        }

        // Only add generic JSON instructions when no system prompt provides its own format.
        if context.system_prompt.is_none() {
            prompt.push_str(
                "Respond with a JSON object containing your result. \
                 The object should have at minimum a \"result\" key with your output \
                 and a \"summary\" key with a brief summary.",
            );
        }

        prompt
    }

    /// Build the command arguments for `claude` CLI.
    fn build_args(&self, prompt: &str) -> Vec<String> {
        let mut args = vec![
            "--model".to_string(),
            self.model.clone(),
            "--permission-mode".to_string(),
            self.permission_mode.clone(),
            "--output-format".to_string(),
            "json".to_string(),
        ];

        if let Some(turns) = self.max_turns {
            args.push("--max-turns".to_string());
            args.push(turns.to_string());
        }

        for tool in &self.allowed_tools {
            args.push("--allowedTools".to_string());
            args.push(tool.clone());
        }

        args.push("-p".to_string());
        args.push(prompt.to_string());

        args
    }
}

#[async_trait]
impl AgentExecutor for ClaudeExecutor {
    async fn execute(
        &self,
        task: &Task,
        context: &ExecutionContext,
    ) -> Result<TaskResult, PanopticonError> {
        let prompt = Self::build_prompt(task, context);
        let args = self.build_args(&prompt);

        let mut cmd = tokio::process::Command::new("claude");
        cmd.args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());

        if let Some(dir) = &context.working_dir {
            cmd.current_dir(dir);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| PanopticonError::Internal(format!("Failed to spawn claude CLI: {e}")))?;

        if !output.status.success() {
            return Err(PanopticonError::Internal(format!(
                "claude CLI exited with {}",
                output.status,
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse the JSON output from claude CLI.
        // The --output-format json wraps the response in a JSON object with a "result" field.
        let parsed: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
            PanopticonError::Serialization(format!(
                "Failed to parse claude output: {e}\nRaw output: {stdout}"
            ))
        })?;

        // Extract the actual result text from the claude JSON envelope.
        let result_text = parsed
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| stdout.as_ref());

        // Strip markdown code fences (```json ... ``` or ``` ... ```) if present.
        let cleaned = strip_code_fences(result_text);

        // Try to parse the result text as JSON; if it fails, wrap it as a string value.
        let output_value: serde_json::Value = serde_json::from_str(&cleaned)
            .unwrap_or_else(|_| serde_json::json!({ "result": result_text, "summary": "" }));

        Ok(TaskResult {
            task_id: task.id,
            agent_id: Uuid::nil(), // Will be set by the caller
            output: output_value,
            completed_at: Utc::now(),
            resource_consumed: 0.0,
        })
    }

    async fn health_check(&self) -> Result<bool, PanopticonError> {
        let output = tokio::process::Command::new("claude")
            .arg("--version")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| PanopticonError::Internal(format!("claude CLI not found: {e}")))?;

        Ok(output.status.success())
    }

    fn name(&self) -> &str {
        "ClaudeExecutor"
    }
}

/// Strip markdown code fences from a string.
/// Handles ```json\n...\n```, ```\n...\n```, and bare JSON.
fn strip_code_fences(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // Skip optional language tag on the first line.
        let rest = if let Some(after_newline) = rest.find('\n') {
            &rest[after_newline + 1..]
        } else {
            rest
        };
        // Strip trailing ```
        let rest = rest.strip_suffix("```").unwrap_or(rest);
        rest.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionContext;

    #[test]
    fn test_build_prompt_default() {
        let task = Task::new("Test task", "Do something useful");
        let ctx = ExecutionContext::default();
        let prompt = ClaudeExecutor::build_prompt(&task, &ctx);
        assert!(prompt.contains("Test task"));
        assert!(prompt.contains("Do something useful"));
        assert!(prompt.contains("JSON object"));
    }

    #[test]
    fn test_build_prompt_with_system_prompt_skips_generic_json() {
        let task = Task::new("Test task", "Do something useful");
        let ctx = ExecutionContext {
            system_prompt: Some("Custom instructions here".to_string()),
            ..Default::default()
        };
        let prompt = ClaudeExecutor::build_prompt(&task, &ctx);
        assert!(prompt.contains("Custom instructions here"));
        assert!(!prompt.contains("JSON object"));
    }

    #[test]
    fn test_build_args() {
        let executor = ClaudeExecutor::default().with_model("opus");
        let args = executor.build_args("hello");
        assert!(args.contains(&"opus".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"json".to_string()));
        assert!(args.contains(&"-p".to_string()));
    }

    #[test]
    fn test_strip_code_fences_json() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(strip_code_fences(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_strip_code_fences_bare() {
        let input = "```\n{\"key\": \"value\"}\n```";
        assert_eq!(strip_code_fences(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_strip_code_fences_none() {
        let input = "{\"key\": \"value\"}";
        assert_eq!(strip_code_fences(input), "{\"key\": \"value\"}");
    }
}
