use crate::types::{Agent, Task};

/// Filters and ranks candidate agents for a given task.
pub struct CapabilityMatcher {
    /// Minimum composite reputation score required.
    pub reputation_threshold: f64,
}

impl CapabilityMatcher {
    pub fn new(reputation_threshold: f64) -> Self {
        Self {
            reputation_threshold,
        }
    }

    /// Return agents that meet all requirements: capabilities, reputation, availability.
    pub fn filter_agents(&self, task: &Task, agents: &[Agent]) -> Vec<Agent> {
        let mut candidates: Vec<Agent> = agents
            .iter()
            .filter(|a| self.meets_capabilities(task, a))
            .filter(|a| self.meets_reputation(a))
            .filter(|a| self.is_available(a))
            .cloned()
            .collect();

        self.rank_candidates(task, &mut candidates);
        candidates
    }

    /// Check whether an agent has all required capabilities for a task.
    fn meets_capabilities(&self, task: &Task, agent: &Agent) -> bool {
        task.required_capabilities
            .iter()
            .all(|cap| agent.has_capability(cap))
    }

    /// Check whether an agent's composite reputation meets the threshold.
    fn meets_reputation(&self, agent: &Agent) -> bool {
        agent.reputation.composite() >= self.reputation_threshold
    }

    /// Check whether the agent is available and has capacity.
    fn is_available(&self, agent: &Agent) -> bool {
        agent.available && (agent.active_task_ids.len() as u32) < agent.max_concurrent_tasks
    }

    /// Rank candidates by composite score: sum of proficiency across required capabilities
    /// weighted by reputation composite. Sorts descending (best first).
    fn rank_candidates(&self, task: &Task, candidates: &mut [Agent]) {
        candidates.sort_by(|a, b| {
            let score_a = self.candidate_score(task, a);
            let score_b = self.candidate_score(task, b);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn candidate_score(&self, task: &Task, agent: &Agent) -> f64 {
        let proficiency_sum: f64 = task
            .required_capabilities
            .iter()
            .map(|cap| agent.capability_proficiency(cap))
            .sum();
        proficiency_sum * agent.reputation.composite()
    }
}

/// Convenience functions that mirror the task description.
impl CapabilityMatcher {
    pub fn filter_by_capabilities<'a>(&self, task: &Task, agents: &'a [Agent]) -> Vec<&'a Agent> {
        agents
            .iter()
            .filter(|a| self.meets_capabilities(task, a))
            .collect()
    }

    pub fn filter_by_reputation<'a>(&self, agents: &'a [Agent], threshold: f64) -> Vec<&'a Agent> {
        agents
            .iter()
            .filter(|a| a.reputation.composite() >= threshold)
            .collect()
    }

    pub fn filter_by_availability<'a>(&self, agents: &'a [Agent]) -> Vec<&'a Agent> {
        agents.iter().filter(|a| self.is_available(a)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::types::{Capability, CapabilityRegistry, ReputationScore};

    fn make_agent(name: &str, caps: &[(&str, f64)], reputation: f64, available: bool) -> Agent {
        let mut agent = Agent::new(name);
        agent.capabilities = CapabilityRegistry {
            capabilities: caps
                .iter()
                .map(|(n, p)| Capability {
                    name: n.to_string(),
                    proficiency: *p,
                    certified: true,
                    last_verified: Some(Utc::now()),
                })
                .collect(),
        };
        agent.reputation = ReputationScore {
            completion: reputation,
            quality: reputation,
            reliability: reputation,
            safety: reputation,
            behavioral: reputation,
        };
        agent.available = available;
        agent
    }

    #[test]
    fn test_filter_agents_by_capability() {
        let matcher = CapabilityMatcher::new(0.3);
        let task = Task::new("test", "desc").with_capabilities(vec!["nlp".into()]);
        let agents = vec![
            make_agent("a1", &[("nlp", 0.9)], 0.8, true),
            make_agent("a2", &[("vision", 0.9)], 0.8, true),
        ];

        let result = matcher.filter_agents(&task, &agents);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "a1");
    }

    #[test]
    fn test_filter_agents_by_reputation() {
        let matcher = CapabilityMatcher::new(0.7);
        let task = Task::new("test", "desc").with_capabilities(vec!["nlp".into()]);
        let agents = vec![
            make_agent("a1", &[("nlp", 0.9)], 0.8, true),
            make_agent("a2", &[("nlp", 0.9)], 0.3, true),
        ];

        let result = matcher.filter_agents(&task, &agents);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "a1");
    }

    #[test]
    fn test_filter_agents_by_availability() {
        let matcher = CapabilityMatcher::new(0.3);
        let task = Task::new("test", "desc").with_capabilities(vec!["nlp".into()]);
        let agents = vec![
            make_agent("a1", &[("nlp", 0.9)], 0.8, true),
            make_agent("a2", &[("nlp", 0.9)], 0.8, false),
        ];

        let result = matcher.filter_agents(&task, &agents);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "a1");
    }

    #[test]
    fn test_ranking_by_proficiency_and_reputation() {
        let matcher = CapabilityMatcher::new(0.3);
        let task = Task::new("test", "desc").with_capabilities(vec!["nlp".into()]);
        let agents = vec![
            make_agent("low", &[("nlp", 0.5)], 0.5, true),
            make_agent("high", &[("nlp", 0.9)], 0.9, true),
            make_agent("mid", &[("nlp", 0.7)], 0.7, true),
        ];

        let result = matcher.filter_agents(&task, &agents);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "high");
        assert_eq!(result[1].name, "mid");
        assert_eq!(result[2].name, "low");
    }

    #[test]
    fn test_empty_capabilities_matches_all() {
        let matcher = CapabilityMatcher::new(0.0);
        let task = Task::new("test", "desc"); // no required capabilities
        let agents = vec![
            make_agent("a1", &[], 0.5, true),
            make_agent("a2", &[("nlp", 0.9)], 0.5, true),
        ];

        let result = matcher.filter_agents(&task, &agents);
        assert_eq!(result.len(), 2);
    }
}
