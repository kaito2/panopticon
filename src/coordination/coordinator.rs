use tokio::sync::{mpsc, watch};

use super::response::{ResponseAction, ResponsePlan};
use super::trigger::{CoordinationTrigger, ExternalTrigger, InternalTrigger};

/// The coordinator receives triggers and maps them to response plans.
pub struct Coordinator {
    trigger_rx: mpsc::Receiver<CoordinationTrigger>,
    shutdown_rx: watch::Receiver<bool>,
    response_tx: mpsc::Sender<ResponsePlan>,
}

impl Coordinator {
    pub fn new(
        trigger_rx: mpsc::Receiver<CoordinationTrigger>,
        shutdown_rx: watch::Receiver<bool>,
        response_tx: mpsc::Sender<ResponsePlan>,
    ) -> Self {
        Self {
            trigger_rx,
            shutdown_rx,
            response_tx,
        }
    }

    /// Run the coordination loop until shutdown is signalled.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(trigger) = self.trigger_rx.recv() => {
                    let plan = Self::handle_trigger(&trigger);
                    tracing::info!("Coordination response: {}", plan.justification);
                    if let Err(e) = self.response_tx.send(plan).await {
                        tracing::error!("Failed to send response plan: {}", e);
                    }
                }
                Ok(()) = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        tracing::info!("Coordinator shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Map a trigger to a response plan.
    pub fn handle_trigger(trigger: &CoordinationTrigger) -> ResponsePlan {
        match trigger {
            CoordinationTrigger::External(ext) => Self::handle_external(ext),
            CoordinationTrigger::Internal(int) => Self::handle_internal(int),
        }
    }

    fn handle_external(trigger: &ExternalTrigger) -> ResponsePlan {
        match trigger {
            ExternalTrigger::TaskSpecChanged { task_id } => ResponsePlan::new(
                "Task specification changed; redecompose to reflect new requirements",
            )
            .with_action(ResponseAction::Redecompose { task_id: *task_id }),

            ExternalTrigger::ResourceFluctuation {
                resource_name,
                delta,
            } => {
                let justification = format!(
                    "Resource '{}' fluctuated by {:.2}; escalating for review",
                    resource_name, delta
                );
                ResponsePlan::new(justification).with_action(ResponseAction::Escalate {
                    task_id: None,
                    reason: format!("Resource fluctuation: {} delta={:.2}", resource_name, delta),
                })
            }

            ExternalTrigger::PriorityChanged {
                task_id,
                new_priority,
            } => {
                let justification = format!(
                    "Priority changed to {:.2}; adjusting task parameters",
                    new_priority
                );
                ResponsePlan::new(justification).with_action(ResponseAction::AdjustParameters {
                    task_id: *task_id,
                    adjustments: serde_json::json!({ "priority": new_priority }),
                })
            }

            ExternalTrigger::SecurityThreat {
                agent_id,
                description,
            } => {
                let justification = format!(
                    "Security threat from agent {}: {}; terminating and escalating",
                    agent_id, description
                );
                // Terminate all tasks from this agent and escalate.
                let mut plan = ResponsePlan::new(justification);
                plan.add_action(ResponseAction::Escalate {
                    task_id: None,
                    reason: format!("Security threat: {}", description),
                });
                plan
            }
        }
    }

    fn handle_internal(trigger: &InternalTrigger) -> ResponsePlan {
        match trigger {
            InternalTrigger::PerformanceDegraded {
                task_id,
                agent_id,
                metric,
                value,
            } => {
                let justification = format!(
                    "Performance degraded for task {} on agent {}: {} = {:.2}; redelegating",
                    task_id, agent_id, metric, value
                );
                ResponsePlan::new(justification).with_action(ResponseAction::Redelegate {
                    task_id: *task_id,
                    from_agent_id: *agent_id,
                })
            }

            InternalTrigger::BudgetExceeded {
                task_id,
                consumed,
                limit,
            } => {
                let justification = format!(
                    "Budget exceeded for task {}: consumed {:.2} / limit {:.2}; terminating",
                    task_id, consumed, limit
                );
                ResponsePlan::new(justification).with_action(ResponseAction::Terminate {
                    task_id: *task_id,
                    reason: format!("Budget exceeded: {:.2} / {:.2}", consumed, limit),
                })
            }

            InternalTrigger::VerificationFailed { task_id, reason } => {
                let justification = format!(
                    "Verification failed for task {}: {}; redecomposing and escalating",
                    task_id, reason
                );
                let mut plan = ResponsePlan::new(justification);
                plan.add_action(ResponseAction::Redecompose { task_id: *task_id });
                plan.add_action(ResponseAction::Escalate {
                    task_id: Some(*task_id),
                    reason: format!("Verification failed: {}", reason),
                });
                plan
            }

            InternalTrigger::AgentUnresponsive { agent_id } => {
                let justification = format!("Agent {} is unresponsive; escalating", agent_id);
                ResponsePlan::new(justification).with_action(ResponseAction::Escalate {
                    task_id: None,
                    reason: format!("Agent {} unresponsive", agent_id),
                })
            }
        }
    }
}

/// Execute a response plan by logging each action.
/// In a real system this would dispatch to actual subsystems.
pub fn execute_response(plan: &ResponsePlan) {
    tracing::info!("Executing response plan: {}", plan.justification);
    for (i, action) in plan.actions.iter().enumerate() {
        match action {
            ResponseAction::AdjustParameters {
                task_id,
                adjustments,
            } => {
                tracing::info!("[{i}] AdjustParameters: task={task_id} adjustments={adjustments}");
            }
            ResponseAction::Redelegate {
                task_id,
                from_agent_id,
            } => {
                tracing::info!("[{i}] Redelegate: task={task_id} from={from_agent_id}");
            }
            ResponseAction::Redecompose { task_id } => {
                tracing::info!("[{i}] Redecompose: task={task_id}");
            }
            ResponseAction::Escalate { task_id, reason } => {
                tracing::info!("[{i}] Escalate: task={task_id:?} reason={reason}");
            }
            ResponseAction::Terminate { task_id, reason } => {
                tracing::info!("[{i}] Terminate: task={task_id} reason={reason}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_trigger_task_spec_changed() {
        let task_id = Uuid::new_v4();
        let trigger = CoordinationTrigger::External(ExternalTrigger::TaskSpecChanged { task_id });
        let plan = Coordinator::handle_trigger(&trigger);
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(
            plan.actions[0],
            ResponseAction::Redecompose { .. }
        ));
    }

    #[test]
    fn test_trigger_budget_exceeded() {
        let task_id = Uuid::new_v4();
        let trigger = CoordinationTrigger::Internal(InternalTrigger::BudgetExceeded {
            task_id,
            consumed: 150.0,
            limit: 100.0,
        });
        let plan = Coordinator::handle_trigger(&trigger);
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(plan.actions[0], ResponseAction::Terminate { .. }));
    }

    #[test]
    fn test_trigger_verification_failed() {
        let task_id = Uuid::new_v4();
        let trigger = CoordinationTrigger::Internal(InternalTrigger::VerificationFailed {
            task_id,
            reason: "output mismatch".into(),
        });
        let plan = Coordinator::handle_trigger(&trigger);
        assert_eq!(plan.actions.len(), 2);
        assert!(matches!(
            plan.actions[0],
            ResponseAction::Redecompose { .. }
        ));
        assert!(matches!(plan.actions[1], ResponseAction::Escalate { .. }));
    }

    #[test]
    fn test_trigger_agent_unresponsive() {
        let agent_id = Uuid::new_v4();
        let trigger =
            CoordinationTrigger::Internal(InternalTrigger::AgentUnresponsive { agent_id });
        let plan = Coordinator::handle_trigger(&trigger);
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(plan.actions[0], ResponseAction::Escalate { .. }));
    }

    #[test]
    fn test_trigger_performance_degraded() {
        let task_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let trigger = CoordinationTrigger::Internal(InternalTrigger::PerformanceDegraded {
            task_id,
            agent_id,
            metric: "latency".into(),
            value: 5000.0,
        });
        let plan = Coordinator::handle_trigger(&trigger);
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(plan.actions[0], ResponseAction::Redelegate { .. }));
    }

    #[test]
    fn test_trigger_security_threat() {
        let agent_id = Uuid::new_v4();
        let trigger = CoordinationTrigger::External(ExternalTrigger::SecurityThreat {
            agent_id,
            description: "data exfiltration attempt".into(),
        });
        let plan = Coordinator::handle_trigger(&trigger);
        assert!(!plan.actions.is_empty());
        assert!(matches!(plan.actions[0], ResponseAction::Escalate { .. }));
    }

    #[test]
    fn test_trigger_priority_changed() {
        let task_id = Uuid::new_v4();
        let trigger = CoordinationTrigger::External(ExternalTrigger::PriorityChanged {
            task_id,
            new_priority: 0.9,
        });
        let plan = Coordinator::handle_trigger(&trigger);
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(
            plan.actions[0],
            ResponseAction::AdjustParameters { .. }
        ));
    }

    #[test]
    fn test_trigger_resource_fluctuation() {
        let trigger = CoordinationTrigger::External(ExternalTrigger::ResourceFluctuation {
            resource_name: "gpu_memory".into(),
            delta: -0.3,
        });
        let plan = Coordinator::handle_trigger(&trigger);
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(plan.actions[0], ResponseAction::Escalate { .. }));
    }

    #[tokio::test]
    async fn test_coordinator_lifecycle() {
        let (trigger_tx, trigger_rx) = mpsc::channel(16);
        let (response_tx, mut response_rx) = mpsc::channel(16);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let coordinator = Coordinator::new(trigger_rx, shutdown_rx, response_tx);
        tokio::spawn(coordinator.run());

        // Send a trigger
        let task_id = Uuid::new_v4();
        trigger_tx
            .send(CoordinationTrigger::External(
                ExternalTrigger::TaskSpecChanged { task_id },
            ))
            .await
            .unwrap();

        // Receive the response plan
        let plan = response_rx.recv().await.unwrap();
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(
            plan.actions[0],
            ResponseAction::Redecompose { .. }
        ));

        // Shutdown
        let _ = shutdown_tx.send(true);
    }

    #[tokio::test]
    async fn test_coordinator_shutdown() {
        let (_trigger_tx, trigger_rx) = mpsc::channel::<CoordinationTrigger>(16);
        let (response_tx, _response_rx) = mpsc::channel(16);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let coordinator = Coordinator::new(trigger_rx, shutdown_rx, response_tx);
        let handle = tokio::spawn(coordinator.run());

        let _ = shutdown_tx.send(true);

        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("Coordinator should shut down within timeout")
            .expect("Task should not panic");
    }
}
