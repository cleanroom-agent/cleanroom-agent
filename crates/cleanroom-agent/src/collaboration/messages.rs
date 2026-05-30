//! Agent-to-agent message protocol (docs/13 §3).
//!
//! Wraps the database-layer `AgentMessage` and `MessageType` from cleanroom-db
//! and adds higher-level message construction and dispatch helpers for the
//! collaboration system.

use std::sync::Arc;

use cleanroom_db::{AgentMessageRepository, Database};

/// Re-export DB-level message types for convenience.
pub use cleanroom_db::{AgentMessage, MessageType};

/// High-level message sender that wraps the database repository.
///
/// Provides convenience methods for sending common message patterns
/// between agents in the collaboration system.
pub struct MessageSender {
    repo: AgentMessageRepository,
}

impl MessageSender {
    /// Create a new message sender.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            repo: AgentMessageRepository::new(db.connection_arc()),
        }
    }

    /// Send a generic message.
    pub fn send(&self, msg: &AgentMessage) -> cleanroom_db::DbResult<()> {
        self.repo.send(msg)
    }

    /// Notify an agent that a dependency has been resolved.
    pub fn notify_dependency_resolved(
        &self,
        from: &str,
        to: &str,
        task_id: &str,
    ) -> cleanroom_db::DbResult<()> {
        let msg = AgentMessage::dependency_resolved(from, to, task_id);
        self.repo.send(&msg)
    }

    /// Send a review request to an agent.
    pub fn request_review(
        &self,
        from: &str,
        to: &str,
        entity_uri: &str,
        issues: Vec<String>,
    ) -> cleanroom_db::DbResult<()> {
        let msg = AgentMessage::review_request(from, to, entity_uri, issues);
        self.repo.send(&msg)
    }

    /// Propose a symbol name to another agent.
    pub fn propose_symbol(
        &self,
        from: &str,
        to: &str,
        sdef_uri: &str,
        proposed_name: &str,
    ) -> cleanroom_db::DbResult<()> {
        let msg = AgentMessage::symbol_proposal(from, to, sdef_uri, proposed_name);
        self.repo.send(&msg)
    }

    /// Send a clarification request.
    pub fn request_clarification(
        &self,
        from: &str,
        to: &str,
        entity_uri: &str,
        question: &str,
    ) -> cleanroom_db::DbResult<()> {
        let msg = AgentMessage::clarification_request(from, to, entity_uri, question);
        self.repo.send(&msg)
    }

    /// Respond to a clarification request.
    pub fn respond_clarification(
        &self,
        from: &str,
        to: &str,
        entity_uri: &str,
        answer: &str,
    ) -> cleanroom_db::DbResult<()> {
        let msg = AgentMessage::clarification_response(from, to, entity_uri, answer);
        self.repo.send(&msg)
    }

    /// Broadcast a message to all agents.
    pub fn broadcast(
        &self,
        from: &str,
        message_type: MessageType,
        payload: serde_json::Value,
    ) -> cleanroom_db::DbResult<()> {
        let msg = AgentMessage::broadcast(from, message_type, payload);
        self.repo.send(&msg)
    }
}

/// High-level message receiver / poller.
pub struct MessagePoller {
    repo: AgentMessageRepository,
}

impl MessagePoller {
    /// Create a new message poller for an agent.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            repo: AgentMessageRepository::new(db.connection_arc()),
        }
    }

    /// Poll for unread messages.
    pub fn poll(&self, agent_id: &str) -> cleanroom_db::DbResult<Vec<AgentMessage>> {
        self.repo.poll(agent_id)
    }

    /// Mark a message as read.
    pub fn mark_read(&self, message_id: &str) -> cleanroom_db::DbResult<()> {
        self.repo.mark_read(message_id)
    }

    /// Mark all messages for an agent as read.
    pub fn mark_all_read(&self, agent_id: &str) -> cleanroom_db::DbResult<usize> {
        self.repo.mark_all_read(agent_id)
    }
}
