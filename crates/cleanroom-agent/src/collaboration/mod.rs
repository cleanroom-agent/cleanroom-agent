//! Agent collaboration module — multi-agent communication and coordination.
//!
//! Provides the infrastructure for direct agent-to-agent messaging,
//! conflict detection, and health monitoring as defined in docs/13-agent-collaboration.md.

pub mod messages;
pub mod conflict_detector;
pub mod health_monitor;
