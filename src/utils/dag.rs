use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet, VecDeque};

use super::Duty;

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    duties: HashMap<String, Duty>,
    edges: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    pub fn new(duties: Vec<Duty>) -> Self {
        let mut graph = Self {
            duties: HashMap::new(),
            edges: HashMap::new(),
        };

        for duty in duties {
            let name = duty.name.clone();
            
            let deps = duty.metadata
                .as_ref()
                .and_then(|m| m.get("depends_on"))
                .and_then(|d| d.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();
            
            graph.edges.insert(name.clone(), deps);
            graph.duties.insert(name, duty);
        }

        graph
    }

    pub fn topological_sort(&self) -> Result<Vec<Vec<String>>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut reverse_edges: HashMap<String, Vec<String>> = HashMap::new();

        for (node, _) in &self.duties {
            in_degree.insert(node.clone(), 0);
            reverse_edges.insert(node.clone(), Vec::new());
        }

        for (node, deps) in &self.edges {
            for dep in deps {
                if !self.duties.contains_key(dep) {
                    anyhow::bail!(
                        "Duty '{}' depends on '{}' which does not exist",
                        node,
                        dep
                    );
                }
                *in_degree.get_mut(node).unwrap() += 1;
                reverse_edges.get_mut(dep).unwrap().push(node.clone());
            }
        }

        let mut batches: Vec<Vec<String>> = Vec::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        while !queue.is_empty() {
            let mut batch = Vec::new();
            let batch_size = queue.len();

            for _ in 0..batch_size {
                if let Some(node) = queue.pop_front() {
                    batch.push(node.clone());

                    if let Some(dependents) = reverse_edges.get(&node) {
                        for dependent in dependents {
                            let degree = in_degree.get_mut(dependent).unwrap();
                            *degree -= 1;
                            if *degree == 0 {
                                queue.push_back(dependent.clone());
                            }
                        }
                    }
                }
            }

            if !batch.is_empty() {
                batches.push(batch);
            }
        }

        let processed_count: usize = batches.iter().map(|b| b.len()).sum();
        if processed_count != self.duties.len() {
            anyhow::bail!("Circular dependency detected in duty graph");
        }

        Ok(batches)
    }

    pub fn get_duty(&self, name: &str) -> Option<&Duty> {
        self.duties.get(name)
    }

    pub fn get_execution_plan(&self) -> Result<Vec<Vec<Duty>>> {
        let batches = self.topological_sort()?;
        
        let mut plan = Vec::new();
        for batch in batches {
            let mut duty_batch = Vec::new();
            for name in batch {
                if let Some(duty) = self.duties.get(&name) {
                    duty_batch.push(duty.clone());
                }
            }
            plan.push(duty_batch);
        }
        
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_duty(name: &str, depends_on: Vec<&str>) -> Duty {
        Duty {
            id: None,
            name: name.to_string(),
            duty_type: "Test".to_string(),
            backend: "test".to_string(),
            roster_selector: json!({"traits": ["test"]}),
            spec: json!({}),
            status: None,
            metadata: if depends_on.is_empty() {
                None
            } else {
                Some(json!({
                    "depends_on": depends_on.into_iter().map(|s| s.to_string()).collect::<Vec<_>>()
                }))
            },
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_no_dependencies() {
        let duties = vec![
            create_test_duty("a", vec![]),
            create_test_duty("b", vec![]),
            create_test_duty("c", vec![]),
        ];

        let graph = DependencyGraph::new(duties);
        let batches = graph.topological_sort().unwrap();

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 3);
        assert!(batches[0].contains(&"a".to_string()));
        assert!(batches[0].contains(&"b".to_string()));
        assert!(batches[0].contains(&"c".to_string()));
    }

    #[test]
    fn test_linear_dependencies() {
        let duties = vec![
            create_test_duty("a", vec![]),
            create_test_duty("b", vec!["a"]),
            create_test_duty("c", vec!["b"]),
        ];

        let graph = DependencyGraph::new(duties);
        let batches = graph.topological_sort().unwrap();

        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0], vec!["a"]);
        assert_eq!(batches[1], vec!["b"]);
        assert_eq!(batches[2], vec!["c"]);
    }

    #[test]
    fn test_parallel_branches() {
        let duties = vec![
            create_test_duty("a", vec![]),
            create_test_duty("b", vec!["a"]),
            create_test_duty("c", vec!["a"]),
            create_test_duty("d", vec!["b", "c"]),
        ];

        let graph = DependencyGraph::new(duties);
        let batches = graph.topological_sort().unwrap();

        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0], vec!["a"]);
        assert_eq!(batches[1].len(), 2);
        assert!(batches[1].contains(&"b".to_string()));
        assert!(batches[1].contains(&"c".to_string()));
        assert_eq!(batches[2], vec!["d"]);
    }

    #[test]
    fn test_circular_dependency() {
        let duties = vec![
            create_test_duty("a", vec!["b"]),
            create_test_duty("b", vec!["a"]),
        ];

        let graph = DependencyGraph::new(duties);
        let result = graph.topological_sort();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
    }

    #[test]
    fn test_missing_dependency() {
        let duties = vec![
            create_test_duty("a", vec!["nonexistent"]),
        ];

        let graph = DependencyGraph::new(duties);
        let result = graph.topological_sort();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_complex_graph() {
        let duties = vec![
            create_test_duty("bucket", vec![]),
            create_test_duty("cert", vec![]),
            create_test_duty("dns-validation", vec!["cert"]),
            create_test_duty("cdn", vec!["bucket", "cert"]),
            create_test_duty("dns-alias", vec!["cdn"]),
            create_test_duty("iam-user", vec!["bucket"]),
        ];

        let graph = DependencyGraph::new(duties);
        let batches = graph.topological_sort().unwrap();

        assert_eq!(batches[0].len(), 2);
        assert!(batches[0].contains(&"bucket".to_string()));
        assert!(batches[0].contains(&"cert".to_string()));

        let batch1_names: HashSet<_> = batches[1].iter().cloned().collect();
        assert!(batch1_names.contains("dns-validation"));
        assert!(batch1_names.contains("cdn") || batch1_names.contains("iam-user"));
    }
}
