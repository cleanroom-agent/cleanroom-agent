//! Workflow pause/resume signal — runtime control for agents.
//!
//! Uses an `AtomicBool` for the pause flag (lock-free) and a Tokio
//! broadcast channel for waking agents when resumed.
//!
//! # Global Signal
//!
//! The signal is stored in a `OnceLock` so that the MCP server (running
//! in the same process) can access it to serve pause/resume commands.
//!
//! # Usage in Agent Loop
//!
//! ```rust,ignore
//! // Before claiming next task:
//! if signal.is_paused() {
//!     signal.wait_for_resume().await;
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::broadcast;

/// Global signal, set by the orchestrator on `start_workflow()` and
/// read by the MCP server's `pause_workflow` / `resume_workflow` tools.
pub static GLOBAL_SIGNAL: OnceLock<Arc<WorkflowSignal>> = OnceLock::new();

/// Shared signal between Orchestrator and all Agent instances.
///
/// The signal mechanism operates entirely at the runtime level — it does
/// NOT send any prompts to the LLM. Agents simply stop claiming new tasks
/// after finishing their current one, and resume claiming tasks when the
/// signal clears.
#[derive(Debug)]
pub struct WorkflowSignal {
    /// Atomic flag: when true, agents stop claiming new tasks after current one finishes.
    paused: AtomicBool,
    /// Channel: Orchestrator sends "resume" notification to waiting agents.
    resume_tx: broadcast::Sender<()>,
}

impl WorkflowSignal {
    /// Create a new workflow signal (running state).
    pub fn new() -> Arc<Self> {
        let (resume_tx, _) = broadcast::channel(16);
        Arc::new(Self {
            paused: AtomicBool::new(false),
            resume_tx,
        })
    }

    /// Request pause — agents will finish current tasks then stop claiming.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
        tracing::info!("Pause signal sent — agents will drain current tasks");
    }

    /// Resume — agents wake up and continue claiming tasks.
    ///
    /// Sends a broadcast notification to any agents currently waiting
    /// on `wait_for_resume()`.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
        let _ = self.resume_tx.send(());
        tracing::info!("Resume signal sent — agents continuing");
    }

    /// Check if workflow is paused. Agents call this after completing a
    /// task, before claiming the next one.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Block until resume signal is received.
    ///
    /// Agents call this when `should_pause()` returns true and no tasks
    /// are available. Returns immediately if already resumed.
    pub async fn wait_for_resume(&self) {
        if !self.is_paused() {
            return;
        }
        let mut rx = self.resume_tx.subscribe();
        // If paused flag was cleared between the check and subscribe,
        // we might miss the broadcast. Re-check.
        if !self.is_paused() {
            return;
        }
        tracing::info!("Agent paused — waiting for resume signal");
        let _ = rx.recv().await;
        tracing::info!("Agent resumed — continuing task loop");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_initial_state() {
        let signal = WorkflowSignal::new();
        assert!(!signal.is_paused());
    }

    #[test]
    fn test_signal_pause_resume() {
        let signal = WorkflowSignal::new();
        signal.pause();
        assert!(signal.is_paused());
        signal.resume();
        assert!(!signal.is_paused());
    }
}
