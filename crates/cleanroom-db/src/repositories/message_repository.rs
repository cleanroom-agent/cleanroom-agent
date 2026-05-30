//! Agent message repository — direct agent-to-agent communication.
//!
//! Stores and retrieves messages between agents in the collaboration system.
//! Messages are persisted in the `agent_messages` table and support polling
//! for unread messages with optional filtering.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::instrument;

use crate::error::{DbError, DbResult};

/// Direct agent-to-agent message (docs/13 §3.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub message_id: String,
    pub from: String,
    pub to: String,
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: String,
    pub read: bool,
}

/// Message types for agent communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum MessageType {
    /// "I've completed task X, your dependency is now satisfied"
    DependencyResolved { task_id: String },
    /// "I found an issue with your output"
    ReviewRequest {
        entity_uri: String,
        issues: Vec<String>,
    },
    /// "Please confirm this symbol assignment"
    SymbolProposal {
        sdef_uri: String,
        proposed_name: String,
    },
    /// "I need more info about X"
    ClarificationRequest {
        entity_uri: String,
        question: String,
    },
    /// "Here's the info you requested"
    ClarificationResponse {
        entity_uri: String,
        answer: String,
    },
}

impl AgentMessage {
    /// Create a DependencyResolved notification.
    pub fn dependency_resolved(
        from: &str,
        to: &str,
        task_id: &str,
    ) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            message_type: MessageType::DependencyResolved {
                task_id: task_id.to_string(),
            },
            payload: serde_json::json!({}),
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
        }
    }

    /// Create a ReviewRequest notification.
    pub fn review_request(
        from: &str,
        to: &str,
        entity_uri: &str,
        issues: Vec<String>,
    ) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            message_type: MessageType::ReviewRequest {
                entity_uri: entity_uri.to_string(),
                issues: issues.clone(),
            },
            payload: serde_json::json!({ "issues": issues }),
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
        }
    }

    /// Create a SymbolProposal.
    pub fn symbol_proposal(
        from: &str,
        to: &str,
        sdef_uri: &str,
        proposed_name: &str,
    ) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            message_type: MessageType::SymbolProposal {
                sdef_uri: sdef_uri.to_string(),
                proposed_name: proposed_name.to_string(),
            },
            payload: serde_json::json!({
                "sdef_uri": sdef_uri,
                "proposed_name": proposed_name,
            }),
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
        }
    }

    /// Create a ClarificationRequest.
    pub fn clarification_request(
        from: &str,
        to: &str,
        entity_uri: &str,
        question: &str,
    ) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            message_type: MessageType::ClarificationRequest {
                entity_uri: entity_uri.to_string(),
                question: question.to_string(),
            },
            payload: serde_json::json!({
                "entity_uri": entity_uri,
                "question": question,
            }),
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
        }
    }

    /// Create a ClarificationResponse.
    pub fn clarification_response(
        from: &str,
        to: &str,
        entity_uri: &str,
        answer: &str,
    ) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            message_type: MessageType::ClarificationResponse {
                entity_uri: entity_uri.to_string(),
                answer: answer.to_string(),
            },
            payload: serde_json::json!({
                "entity_uri": entity_uri,
                "answer": answer,
            }),
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
        }
    }

    /// Convert to a broadcast message (to = "broadcast").
    pub fn broadcast(from: &str, message_type: MessageType, payload: serde_json::Value) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: "broadcast".to_string(),
            message_type,
            payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
        }
    }
}

/// Repository for agent message operations.
pub struct AgentMessageRepository {
    conn: Arc<Mutex<Connection>>,
}

impl AgentMessageRepository {
    /// Create a new repository.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Send and persist a message.
    #[instrument(skip_all)]
    pub fn send(&self, msg: &AgentMessage) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO agent_messages (message_id, from_agent, to_agent, message_type, payload, timestamp, read)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![
                msg.message_id,
                msg.from,
                msg.to,
                serde_json::to_string(&msg.message_type)
                    .map_err(|e| DbError::SerializationError(e.to_string()))?,
                serde_json::to_string(&msg.payload)
                    .map_err(|e| DbError::SerializationError(e.to_string()))?,
                msg.timestamp,
            ],
        )
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Fetch unread messages for an agent.
    #[instrument(skip_all)]
    pub fn poll(&self, agent_id: &str) -> DbResult<Vec<AgentMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT message_id, from_agent, to_agent, message_type, payload, timestamp, read
                 FROM agent_messages
                 WHERE (to_agent = ?1 OR to_agent = 'broadcast')
                   AND read = 0
                 ORDER BY timestamp ASC",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let messages = stmt
            .query_map(params![agent_id], |row| {
                let type_str: String = row.get(3)?;
                let payload_str: String = row.get(4)?;
                let read_flag: bool = row.get(6)?;
                Ok(AgentMessage {
                    message_id: row.get(0)?,
                    from: row.get(1)?,
                    to: row.get(2)?,
                    message_type: serde_json::from_str(&type_str).unwrap_or_else(|_| {
                        MessageType::DependencyResolved {
                            task_id: "unknown".to_string(),
                        }
                    }),
                    payload: serde_json::from_str(&payload_str)
                        .unwrap_or(serde_json::json!({})),
                    timestamp: row.get(5)?,
                    read: read_flag,
                })
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages)
    }

    /// Mark a message as read.
    #[instrument(skip_all)]
    pub fn mark_read(&self, message_id: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        let rows = conn
            .execute(
                "UPDATE agent_messages SET read = 1 WHERE message_id = ?1",
                params![message_id],
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        if rows == 0 {
            return Err(DbError::NotFound {
                resource: "agent_message",
                field: "message_id",
                value: message_id.to_string(),
            });
        }
        Ok(())
    }

    /// Mark all messages for an agent as read.
    #[instrument(skip_all)]
    pub fn mark_all_read(&self, agent_id: &str) -> DbResult<usize> {
        let conn = self.conn.lock().unwrap();
        let rows = conn
            .execute(
                "UPDATE agent_messages SET read = 1 WHERE to_agent = ?1 AND read = 0",
                params![agent_id],
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows)
    }

    /// Delete a message by ID.
    #[instrument(skip_all)]
    pub fn delete(&self, message_id: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        let rows = conn
            .execute(
                "DELETE FROM agent_messages WHERE message_id = ?1",
                params![message_id],
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        if rows == 0 {
            return Err(DbError::NotFound {
                resource: "agent_message",
                field: "message_id",
                value: message_id.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    fn setup() -> (Database, AgentMessageRepository) {
        let db = Database::in_memory().unwrap();
        // Create agent_messages table manually for in-memory tests
        let conn = db.connection();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_messages (
                message_id TEXT PRIMARY KEY,
                from_agent TEXT NOT NULL,
                to_agent TEXT NOT NULL,
                message_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                read BOOLEAN NOT NULL DEFAULT 0
            );",
        )
        .unwrap();
        drop(conn);
        let repo = AgentMessageRepository::new(db.connection_arc());
        (db, repo)
    }

    #[test]
    fn test_send_and_poll() {
        let (_, repo) = setup();
        let msg = AgentMessage::dependency_resolved("agent-1", "agent-2", "task-123");

        repo.send(&msg).unwrap();

        let messages = repo.poll("agent-2").unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_id, msg.message_id);
        assert_eq!(messages[0].from, "agent-1");
        assert!(!messages[0].read);
    }

    #[test]
    fn test_broadcast_message() {
        let (_, repo) = setup();
        let msg = AgentMessage::broadcast(
            "orchestrator",
            MessageType::DependencyResolved {
                task_id: "task-456".to_string(),
            },
            serde_json::json!({"note": "all agents notified"}),
        );

        repo.send(&msg).unwrap();

        // Broadcast should be visible to any agent poll
        let messages_agent1 = repo.poll("agent-1").unwrap();
        let messages_agent2 = repo.poll("agent-2").unwrap();
        assert_eq!(messages_agent1.len(), 1);
        assert_eq!(messages_agent2.len(), 1);
    }

    #[test]
    fn test_mark_read() {
        let (_, repo) = setup();
        let msg = AgentMessage::dependency_resolved("agent-1", "agent-2", "task-123");

        repo.send(&msg).unwrap();
        repo.mark_read(&msg.message_id).unwrap();

        // Should no longer appear in poll
        let messages = repo.poll("agent-2").unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_mark_all_read() {
        let (_, repo) = setup();
        let msg1 = AgentMessage::dependency_resolved("agent-1", "agent-2", "task-a");
        let msg2 = AgentMessage::dependency_resolved("agent-1", "agent-2", "task-b");
        repo.send(&msg1).unwrap();
        repo.send(&msg2).unwrap();

        let marked = repo.mark_all_read("agent-2").unwrap();
        assert_eq!(marked, 2);

        let remaining = repo.poll("agent-2").unwrap();
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_delete() {
        let (_, repo) = setup();
        let msg = AgentMessage::dependency_resolved("agent-1", "agent-2", "task-123");

        repo.send(&msg).unwrap();
        repo.delete(&msg.message_id).unwrap();

        let messages = repo.poll("agent-2").unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_review_request_message() {
        let (_, repo) = setup();
        let msg = AgentMessage::review_request(
            "reviewer-1",
            "consumer-1",
            "sdef://model/User",
            vec!["Missing field: email".to_string()],
        );

        repo.send(&msg).unwrap();

        let messages = repo.poll("consumer-1").unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0].message_type {
            MessageType::ReviewRequest { entity_uri, issues } => {
                assert_eq!(entity_uri, "sdef://model/User");
                assert_eq!(issues.len(), 1);
            }
            _ => panic!("Expected ReviewRequest"),
        }
    }

    #[test]
    fn test_symbol_proposal_message() {
        let (_, repo) = setup();
        let msg = AgentMessage::symbol_proposal(
            "producer-1",
            "consumer-1",
            "sdef://symbol/user_callback",
            "user_callback_fn",
        );

        repo.send(&msg).unwrap();

        let messages = repo.poll("consumer-1").unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0].message_type {
            MessageType::SymbolProposal {
                sdef_uri,
                proposed_name,
            } => {
                assert_eq!(sdef_uri, "sdef://symbol/user_callback");
                assert_eq!(proposed_name, "user_callback_fn");
            }
            _ => panic!("Expected SymbolProposal"),
        }
    }
}
