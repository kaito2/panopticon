use serde::{Deserialize, Serialize};

/// A single message in the conversation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Manages the conversation history for a REPL session.
pub struct Session {
    messages: Vec<Message>,
    max_messages: usize,
}

impl Session {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
        }
    }

    /// Push a user message.
    pub fn push_user(&mut self, content: &str) {
        self.messages.push(Message {
            role: Role::User,
            content: content.to_string(),
        });
        self.trim();
    }

    /// Push an assistant message.
    pub fn push_assistant(&mut self, content: &str) {
        self.messages.push(Message {
            role: Role::Assistant,
            content: content.to_string(),
        });
        self.trim();
    }

    /// Format the conversation history for inclusion in a Claude prompt.
    pub fn format_for_claude(&self) -> String {
        if self.messages.is_empty() {
            return String::new();
        }

        let mut out = String::from("Conversation history:\n");
        for msg in &self.messages {
            let prefix = match msg.role {
                Role::User => "User",
                Role::Assistant => "Assistant",
            };
            out.push_str(&format!("{prefix}: {}\n", msg.content));
        }
        out
    }

    fn trim(&mut self) {
        if self.messages.len() > self.max_messages {
            let excess = self.messages.len() - self.max_messages;
            self.messages.drain(..excess);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_push_and_trim() {
        let mut session = Session::new(3);
        session.push_user("hello");
        session.push_assistant("hi");
        session.push_user("how are you");
        session.push_assistant("good");
        // Should have trimmed to 3 messages
        assert_eq!(session.messages.len(), 3);
        assert_eq!(session.messages[0].role, Role::Assistant);
        assert_eq!(session.messages[0].content, "hi");
    }

    #[test]
    fn test_format_for_claude() {
        let mut session = Session::new(20);
        session.push_user("plan a website");
        session.push_assistant("I'll create a plan for you.");
        let formatted = session.format_for_claude();
        assert!(formatted.contains("User: plan a website"));
        assert!(formatted.contains("Assistant: I'll create a plan for you."));
    }

    #[test]
    fn test_empty_session_format() {
        let session = Session::new(20);
        assert_eq!(session.format_for_claude(), "");
    }
}
