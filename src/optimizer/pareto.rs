use serde::{Deserialize, Serialize};

/// A candidate solution with its objective values.
/// All values are normalized to [0, 1] where higher is better.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    pub id: String,
    /// Objective values, all oriented so that higher = better.
    pub objectives: Vec<f64>,
}

impl Solution {
    pub fn new(id: impl Into<String>, objectives: Vec<f64>) -> Self {
        Self {
            id: id.into(),
            objectives,
        }
    }
}

/// Returns true if solution `a` dominates solution `b`.
///
/// `a` dominates `b` if `a` is at least as good as `b` in all objectives
/// and strictly better in at least one.
pub fn dominates(a: &Solution, b: &Solution) -> bool {
    assert_eq!(
        a.objectives.len(),
        b.objectives.len(),
        "solutions must have the same number of objectives"
    );

    let mut at_least_one_strictly_better = false;

    for (va, vb) in a.objectives.iter().zip(b.objectives.iter()) {
        if va < vb {
            return false; // a is worse in this objective
        }
        if va > vb {
            at_least_one_strictly_better = true;
        }
    }

    at_least_one_strictly_better
}

/// Compute the Pareto front from a set of solutions.
///
/// Returns only the non-dominated solutions.
pub fn compute_pareto_front(solutions: Vec<Solution>) -> Vec<Solution> {
    let mut front = Vec::new();

    for candidate in solutions {
        // Check if the candidate is dominated by any current member of the front
        let is_dominated = front
            .iter()
            .any(|member: &Solution| dominates(member, &candidate));

        if !is_dominated {
            // Remove any current front members that the candidate dominates
            front.retain(|member| !dominates(&candidate, member));
            front.push(candidate);
        }
    }

    front
}

/// Select the best solution from the Pareto front using weighted scoring.
///
/// `weights` should have the same length as the objective vectors.
/// Returns None if the front is empty.
pub fn select_best<'a>(front: &'a [Solution], weights: &[f64]) -> Option<&'a Solution> {
    front.iter().max_by(|a, b| {
        let score_a: f64 = a
            .objectives
            .iter()
            .zip(weights.iter())
            .map(|(v, w)| v * w)
            .sum();
        let score_b: f64 = b
            .objectives
            .iter()
            .zip(weights.iter())
            .map(|(v, w)| v * w)
            .sum();
        score_a
            .partial_cmp(&score_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dominates_basic() {
        let a = Solution::new("a", vec![0.8, 0.9]);
        let b = Solution::new("b", vec![0.5, 0.6]);
        assert!(dominates(&a, &b));
        assert!(!dominates(&b, &a));
    }

    #[test]
    fn test_no_dominance_tradeoff() {
        let a = Solution::new("a", vec![0.9, 0.3]);
        let b = Solution::new("b", vec![0.3, 0.9]);
        assert!(!dominates(&a, &b));
        assert!(!dominates(&b, &a));
    }

    #[test]
    fn test_equal_solutions_no_dominance() {
        let a = Solution::new("a", vec![0.5, 0.5]);
        let b = Solution::new("b", vec![0.5, 0.5]);
        assert!(!dominates(&a, &b));
        assert!(!dominates(&b, &a));
    }

    #[test]
    fn test_pareto_front_simple() {
        let solutions = vec![
            Solution::new("dominated", vec![0.3, 0.3]),
            Solution::new("front1", vec![0.9, 0.4]),
            Solution::new("front2", vec![0.4, 0.9]),
            Solution::new("also_dominated", vec![0.2, 0.2]),
        ];

        let front = compute_pareto_front(solutions);

        // The front should contain only non-dominated solutions
        assert_eq!(front.len(), 2);
        let ids: Vec<&str> = front.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"front1"));
        assert!(ids.contains(&"front2"));
    }

    #[test]
    fn test_pareto_front_no_dominated() {
        // Verify the non-dominance property: no solution in the front dominates another
        let solutions = vec![
            Solution::new("a", vec![0.9, 0.1, 0.5]),
            Solution::new("b", vec![0.1, 0.9, 0.5]),
            Solution::new("c", vec![0.5, 0.5, 0.9]),
            Solution::new("d", vec![0.3, 0.3, 0.3]),
        ];

        let front = compute_pareto_front(solutions);

        for i in 0..front.len() {
            for j in 0..front.len() {
                if i != j {
                    assert!(
                        !dominates(&front[i], &front[j]),
                        "{} should not dominate {}",
                        front[i].id,
                        front[j].id
                    );
                }
            }
        }
    }

    #[test]
    fn test_pareto_front_all_non_dominated() {
        let solutions = vec![
            Solution::new("a", vec![1.0, 0.0]),
            Solution::new("b", vec![0.0, 1.0]),
        ];

        let front = compute_pareto_front(solutions);
        assert_eq!(front.len(), 2);
    }

    #[test]
    fn test_pareto_front_single_solution() {
        let solutions = vec![Solution::new("only", vec![0.5, 0.5])];
        let front = compute_pareto_front(solutions);
        assert_eq!(front.len(), 1);
    }

    #[test]
    fn test_pareto_front_empty() {
        let solutions: Vec<Solution> = vec![];
        let front = compute_pareto_front(solutions);
        assert!(front.is_empty());
    }

    #[test]
    fn test_select_best_weighted() {
        let front = vec![
            Solution::new("cost_oriented", vec![0.9, 0.3]),
            Solution::new("quality_oriented", vec![0.3, 0.9]),
        ];

        // Weight towards first objective (cost)
        let best = select_best(&front, &[0.8, 0.2]).unwrap();
        assert_eq!(best.id, "cost_oriented");

        // Weight towards second objective (quality)
        let best = select_best(&front, &[0.2, 0.8]).unwrap();
        assert_eq!(best.id, "quality_oriented");
    }

    #[test]
    fn test_select_best_empty_front() {
        let front: Vec<Solution> = vec![];
        assert!(select_best(&front, &[1.0]).is_none());
    }
}
