//! Dependency graph — represents and analyzes dependencies between entities.

use std::collections::{HashMap, HashSet, VecDeque};

/// A node in the dependency graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DepNode {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Node type.
    pub node_type: DepNodeType,
}

/// Type of dependency node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DepNodeType {
    /// Source file.
    File,
    /// Module.
    Module,
    /// Interface.
    Interface,
    /// Entity / data model.
    Entity,
}

/// An edge representing a dependency.
#[derive(Debug, Clone)]
pub struct DepEdge {
    pub from: String,
    pub to: String,
    pub kind: DepEdgeKind,
}

/// Kind of dependency edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepEdgeKind {
    Import,
    Extends,
    Implements,
    Uses,
    References,
}

/// Dependency graph with cycle detection.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// All nodes keyed by ID.
    nodes: HashMap<String, DepNode>,
    /// Adjacency list: node ID → children (dependencies).
    edges: HashMap<String, Vec<DepEdge>>,
    /// Reverse adjacency list: node ID → dependents.
    reverse_edges: HashMap<String, Vec<DepEdge>>,
}

impl DependencyGraph {
    /// Create an empty dependency graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: DepNode) {
        let id = node.id.clone();
        self.nodes.entry(id.clone()).or_insert(node);
        self.edges.entry(id.clone()).or_default();
        self.reverse_edges.entry(id).or_default();
    }

    /// Add a dependency edge.
    pub fn add_edge(&mut self, from: &str, to: &str, kind: DepEdgeKind) {
        // Ensure both nodes exist
        if !self.nodes.contains_key(from) || !self.nodes.contains_key(to) {
            return;
        }
        
        let edge = DepEdge {
            from: from.to_string(),
            to: to.to_string(),
            kind,
        };
        
        self.edges.entry(from.to_string()).or_default().push(edge.clone());
        self.reverse_edges.entry(to.to_string()).or_default().push(edge);
    }

    /// Check if the graph has cycles. Returns all cycles found.
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();
        let mut cycles = Vec::new();
        
        for node_id in self.nodes.keys() {
            if !visited.contains(node_id.as_str()) {
                let mut path = Vec::new();
                self.dfs_cycle(node_id, &mut visited, &mut in_stack, &mut path, &mut cycles);
            }
        }
        
        cycles
    }

    fn dfs_cycle(
        &self,
        node_id: &str,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node_id.to_string());
        in_stack.insert(node_id.to_string());
        path.push(node_id.to_string());

        if let Some(children) = self.edges.get(node_id) {
            for edge in children {
                let child_id = &edge.to;
                if !visited.contains(child_id.as_str()) {
                    self.dfs_cycle(child_id, visited, in_stack, path, cycles);
                } else if in_stack.contains(child_id.as_str()) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|x| x == child_id).unwrap_or(0);
                    let cycle: Vec<String> = path[cycle_start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }

        path.pop();
        in_stack.remove(node_id);
    }

    /// Get upstream dependencies of a node (what this node depends on).
    pub fn upstream(&self, node_id: &str) -> Vec<DepNode> {
        self.edges.get(node_id)
            .map(|edges| {
                edges.iter()
                    .filter_map(|e| self.nodes.get(&e.to))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get downstream dependents (what depends on this node).
    pub fn downstream(&self, node_id: &str) -> Vec<DepNode> {
        self.reverse_edges.get(node_id)
            .map(|edges| {
                edges.iter()
                    .filter_map(|e| self.nodes.get(&e.from))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get topological ordering of all nodes.
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        if !self.detect_cycles().is_empty() {
            return None;
        }

        let mut in_degree: HashMap<String, usize> = self.nodes.keys()
            .map(|k| (k.clone(), 0))
            .collect();
        
        // Count in-degrees
        for (_, edges) in &self.edges {
            for edge in edges {
                *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<String> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut result = Vec::new();
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());
            if let Some(edges) = self.edges.get(&node) {
                for edge in edges {
                    if let Some(deg) = in_degree.get_mut(&edge.to) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(edge.to.clone());
                        }
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None
        }
    }

    /// Get the level of each node (distance from root).
    pub fn levels(&self) -> HashMap<String, usize> {
        let mut levels: HashMap<String, usize> = HashMap::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        // Start with nodes that have no dependencies
        for (id, _) in &self.nodes {
            if self.upstream(id).is_empty() {
                levels.insert(id.clone(), 0);
                queue.push_back(id.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            let current_level = *levels.get(&node).unwrap_or(&0);
            for dep in self.downstream(&node) {
                let new_level = current_level + 1;
                let entry = levels.entry(dep.id.clone()).or_insert(0);
                if new_level > *entry {
                    *entry = new_level;
                }
                queue.push_back(dep.id.clone());
            }
        }

        levels
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|e| e.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, name: &str, node_type: DepNodeType) -> DepNode {
        DepNode {
            id: id.to_string(),
            name: name.to_string(),
            node_type,
        }
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.detect_cycles().is_empty());
    }

    #[test]
    fn test_add_nodes_and_edges() {
        let mut graph = DependencyGraph::new();
        graph.add_node(make_node("a", "Module A", DepNodeType::Module));
        graph.add_node(make_node("b", "Module B", DepNodeType::Module));
        graph.add_edge("a", "b", DepEdgeKind::Import);

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        
        let upstream = graph.upstream("a");
        assert_eq!(upstream.len(), 1);
        assert_eq!(upstream[0].id, "b");
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_node(make_node("a", "A", DepNodeType::Module));
        graph.add_node(make_node("b", "B", DepNodeType::Module));
        graph.add_node(make_node("c", "C", DepNodeType::Module));
        
        graph.add_edge("a", "b", DepEdgeKind::Import);
        graph.add_edge("b", "c", DepEdgeKind::Import);
        graph.add_edge("c", "a", DepEdgeKind::Import); // Cycle!

        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
        assert!(graph.topological_sort().is_none());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::new();
        graph.add_node(make_node("a", "A", DepNodeType::Module));
        graph.add_node(make_node("b", "B", DepNodeType::Module));
        graph.add_node(make_node("c", "C", DepNodeType::Module));
        
        graph.add_edge("c", "a", DepEdgeKind::Import);
        graph.add_edge("b", "a", DepEdgeKind::Import);

        let sorted = graph.topological_sort().unwrap();
        // b and c should come before a
        let pos_a = sorted.iter().position(|x| x == "a").unwrap();
        let pos_b = sorted.iter().position(|x| x == "b").unwrap();
        let pos_c = sorted.iter().position(|x| x == "c").unwrap();
        assert!(pos_b < pos_a);
        assert!(pos_c < pos_a);
    }

    #[test]
    fn test_downstream() {
        let mut graph = DependencyGraph::new();
        graph.add_node(make_node("base", "Base", DepNodeType::Interface));
        graph.add_node(make_node("impl", "Impl", DepNodeType::Module));
        graph.add_edge("impl", "base", DepEdgeKind::Implements);

        let downstream = graph.downstream("base");
        assert_eq!(downstream.len(), 1);
        assert_eq!(downstream[0].id, "impl");
    }

    #[test]
    fn test_levels() {
        let mut graph = DependencyGraph::new();
        graph.add_node(make_node("root", "Root", DepNodeType::Module));
        graph.add_node(make_node("mid", "Mid", DepNodeType::Module));
        graph.add_node(make_node("leaf", "Leaf", DepNodeType::Module));
        
        graph.add_edge("mid", "root", DepEdgeKind::Import);
        graph.add_edge("leaf", "mid", DepEdgeKind::Import);

        let levels = graph.levels();
        assert_eq!(*levels.get("root").unwrap(), 0);
        assert_eq!(*levels.get("mid").unwrap(), 1);
        assert_eq!(*levels.get("leaf").unwrap(), 2);
    }
}