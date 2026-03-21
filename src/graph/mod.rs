//! Audio graph — node-based processing with topological execution.
//!
//! # Architecture
//!
//! - **`AudioNode`** trait: process audio with typed inputs/outputs
//! - **`Graph`**: non-RT builder — add nodes, connect edges
//! - **`ExecutionPlan`**: compiled topological order (Kahn's algorithm)
//! - **`GraphProcessor`**: RT-thread processor with double-buffered plan swap
//! - **`NodeId`**: atomic unique ID generator

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::buffer::AudioBuffer;

// ── NodeId ──────────────────────────────────────────────────────────

static NEXT_NODE_ID: AtomicU32 = AtomicU32::new(1);

/// Unique identifier for a node in the audio graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Generate the next unique node ID.
    pub fn next() -> Self {
        Self(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

// ── AudioNode trait ─────────────────────────────────────────────────

/// Trait for audio processing nodes.
pub trait AudioNode: Send {
    /// Node name / type identifier.
    fn name(&self) -> &str;
    /// Number of input ports.
    fn num_inputs(&self) -> usize;
    /// Number of output ports.
    fn num_outputs(&self) -> usize;
    /// Process one buffer cycle.
    fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer);
    /// Whether this node has finished producing output.
    fn is_finished(&self) -> bool {
        false
    }
}

// ── Connection ──────────────────────────────────────────────────────

/// A directed connection from one node's output to another's input.
#[derive(Debug, Clone)]
pub struct Connection {
    pub from: NodeId,
    pub to: NodeId,
}

// ── Graph (non-RT builder) ──────────────────────────────────────────

/// Non-real-time audio graph builder.
///
/// Add nodes and connections, then `compile()` to produce an `ExecutionPlan`.
pub struct Graph {
    nodes: HashMap<NodeId, Box<dyn AudioNode>>,
    connections: Vec<Connection>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
        }
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, id: NodeId, node: Box<dyn AudioNode>) {
        self.nodes.insert(id, node);
    }

    /// Connect one node's output to another's input.
    pub fn connect(&mut self, from: NodeId, to: NodeId) {
        self.connections.push(Connection { from, to });
    }

    /// Compile the graph into a topologically sorted execution plan.
    pub fn compile(self) -> Result<ExecutionPlan, &'static str> {
        let order = topological_sort(&self.nodes, &self.connections)?;
        let mut input_map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for conn in &self.connections {
            input_map.entry(conn.to).or_default().push(conn.from);
        }

        Ok(ExecutionPlan {
            order,
            nodes: self.nodes,
            input_map,
        })
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of connections.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

// ── ExecutionPlan ───────────────────────────────────────────────────

/// Compiled, topologically sorted execution plan.
pub struct ExecutionPlan {
    order: Vec<NodeId>,
    nodes: HashMap<NodeId, Box<dyn AudioNode>>,
    input_map: HashMap<NodeId, Vec<NodeId>>,
}

impl ExecutionPlan {
    /// Execution order (topologically sorted node IDs).
    pub fn order(&self) -> &[NodeId] {
        &self.order
    }

    /// Check if the last node is finished.
    pub fn is_finished(&self) -> bool {
        self.order
            .last()
            .and_then(|id| self.nodes.get(id))
            .is_some_and(|n| n.is_finished())
    }
}

// ── GraphProcessor (RT-thread) ──────────────────────────────────────

/// Real-time audio graph processor with double-buffered plan swapping.
///
/// The RT thread calls `process()` each buffer cycle. New plans are swapped
/// in from the non-RT thread via `GraphSwapHandle` without blocking.
pub struct GraphProcessor {
    current_plan: Option<ExecutionPlan>,
    pending_plan: Arc<Mutex<Option<ExecutionPlan>>>,
    node_outputs: HashMap<NodeId, AudioBuffer>,
    channels: u32,
    sample_rate: u32,
    buffer_frames: usize,
}

impl GraphProcessor {
    pub fn new(channels: u32, sample_rate: u32, buffer_frames: usize) -> Self {
        Self {
            current_plan: None,
            pending_plan: Arc::new(Mutex::new(None)),
            node_outputs: HashMap::new(),
            channels,
            sample_rate,
            buffer_frames,
        }
    }

    /// Get a handle for the non-RT thread to swap in new plans.
    pub fn swap_handle(&self) -> GraphSwapHandle {
        GraphSwapHandle {
            pending_plan: self.pending_plan.clone(),
        }
    }

    /// Process one audio buffer cycle. Call from the RT thread.
    ///
    /// Returns a reference to the output buffer, or None if no plan is active.
    pub fn process(&mut self) -> Option<&AudioBuffer> {
        // Try to pick up pending plan (non-blocking on contention)
        if let Ok(mut pending) = self.pending_plan.try_lock()
            && let Some(new_plan) = pending.take()
        {
            self.current_plan = Some(new_plan);
            self.node_outputs.clear();
        }

        let plan = self.current_plan.as_mut()?;

        // Process nodes in topological order
        let order: Vec<NodeId> = plan.order.clone();
        for &node_id in &order {
            // Gather inputs
            let inputs: Vec<AudioBuffer> = plan
                .input_map
                .get(&node_id)
                .map(|ids| {
                    ids.iter()
                        .filter_map(|id| self.node_outputs.get(id).cloned())
                        .collect()
                })
                .unwrap_or_default();

            let input_refs: Vec<&AudioBuffer> = inputs.iter().collect();

            let mut output =
                AudioBuffer::silence(self.channels, self.buffer_frames, self.sample_rate);

            if let Some(node) = plan.nodes.get_mut(&node_id) {
                node.process(&input_refs, &mut output);
            }

            self.node_outputs.insert(node_id, output);
        }

        // Return the last node's output
        order.last().and_then(|id| self.node_outputs.get(id))
    }

    /// Whether the current plan's last node is finished.
    pub fn is_finished(&self) -> bool {
        self.current_plan
            .as_ref()
            .is_some_and(|p| p.is_finished())
    }
}

/// Handle for the non-RT thread to swap in new execution plans.
#[derive(Clone)]
pub struct GraphSwapHandle {
    pending_plan: Arc<Mutex<Option<ExecutionPlan>>>,
}

impl GraphSwapHandle {
    /// Swap in a new execution plan. The RT thread will pick it up on its next cycle.
    pub fn swap(&self, new_plan: ExecutionPlan) {
        match self.pending_plan.lock() {
            Ok(mut slot) => {
                *slot = Some(new_plan);
            }
            Err(poisoned) => {
                // Recover from poisoned mutex
                let mut slot = poisoned.into_inner();
                *slot = Some(new_plan);
            }
        }
    }
}

// ── Topological Sort ────────────────────────────────────────────────

fn topological_sort(
    nodes: &HashMap<NodeId, Box<dyn AudioNode>>,
    connections: &[Connection],
) -> Result<Vec<NodeId>, &'static str> {
    // Build adjacency list and in-degree count
    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
    let mut successors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for id in nodes.keys() {
        in_degree.entry(*id).or_insert(0);
        successors.entry(*id).or_default();
    }

    for conn in connections {
        *in_degree.entry(conn.to).or_insert(0) += 1;
        successors.entry(conn.from).or_default().push(conn.to);
    }

    // Kahn's algorithm
    let mut queue: Vec<NodeId> = in_degree
        .iter()
        .filter(|&(_, deg)| *deg == 0)
        .map(|(&id, _)| id)
        .collect();
    // Sort for deterministic ordering
    queue.sort_by_key(|id| id.0);

    let mut order = Vec::with_capacity(nodes.len());

    while let Some(node) = queue.pop() {
        order.push(node);
        if let Some(succs) = successors.get(&node) {
            for &succ in succs {
                if let Some(deg) = in_degree.get_mut(&succ) {
                    *deg -= 1;
                    if *deg == 0 {
                        // Insert in sorted position for determinism
                        let pos = queue.partition_point(|id| id.0 >= succ.0);
                        queue.insert(pos, succ);
                    }
                }
            }
        }
    }

    if order.len() != nodes.len() {
        return Err("cycle detected in audio graph");
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple passthrough node for testing
    struct PassthroughNode;
    impl AudioNode for PassthroughNode {
        fn name(&self) -> &str { "passthrough" }
        fn num_inputs(&self) -> usize { 1 }
        fn num_outputs(&self) -> usize { 1 }
        fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
            if let Some(input) = inputs.first() {
                output.samples.copy_from_slice(&input.samples);
            }
        }
    }

    struct GeneratorNode {
        value: f32,
    }
    impl AudioNode for GeneratorNode {
        fn name(&self) -> &str { "generator" }
        fn num_inputs(&self) -> usize { 0 }
        fn num_outputs(&self) -> usize { 1 }
        fn process(&mut self, _inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
            for s in &mut output.samples {
                *s = self.value;
            }
        }
    }

    struct GainNode { gain: f32, }
    impl AudioNode for GainNode {
        fn name(&self) -> &str { "gain" }
        fn num_inputs(&self) -> usize { 1 }
        fn num_outputs(&self) -> usize { 1 }
        fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
            if let Some(input) = inputs.first() {
                for (i, s) in output.samples.iter_mut().enumerate() {
                    *s = input.samples.get(i).copied().unwrap_or(0.0) * self.gain;
                }
            }
        }
    }

    #[test]
    fn node_id_unique() {
        let a = NodeId::next();
        let b = NodeId::next();
        assert_ne!(a, b);
    }

    #[test]
    fn empty_graph_compiles() {
        let graph = Graph::new();
        let plan = graph.compile().unwrap();
        assert!(plan.order().is_empty());
    }

    #[test]
    fn single_node_graph() {
        let mut graph = Graph::new();
        let id = NodeId::next();
        graph.add_node(id, Box::new(GeneratorNode { value: 0.5 }));
        let plan = graph.compile().unwrap();
        assert_eq!(plan.order().len(), 1);
    }

    #[test]
    fn linear_chain() {
        let mut graph = Graph::new();
        let src = NodeId::next();
        let gain = NodeId::next();
        let out = NodeId::next();

        graph.add_node(src, Box::new(GeneratorNode { value: 1.0 }));
        graph.add_node(gain, Box::new(GainNode { gain: 0.5 }));
        graph.add_node(out, Box::new(PassthroughNode));

        graph.connect(src, gain);
        graph.connect(gain, out);

        let plan = graph.compile().unwrap();
        assert_eq!(plan.order().len(), 3);
        // Generator should come first
        assert_eq!(plan.order()[0], src);
    }

    #[test]
    fn cycle_detected() {
        let mut graph = Graph::new();
        let a = NodeId::next();
        let b = NodeId::next();

        graph.add_node(a, Box::new(PassthroughNode));
        graph.add_node(b, Box::new(PassthroughNode));

        graph.connect(a, b);
        graph.connect(b, a); // cycle!

        assert!(graph.compile().is_err());
    }

    #[test]
    fn graph_processor_no_plan_returns_none() {
        let mut proc = GraphProcessor::new(2, 44100, 1024);
        assert!(proc.process().is_none());
    }

    #[test]
    fn graph_processor_with_plan() {
        let mut graph = Graph::new();
        let src = NodeId::next();
        graph.add_node(src, Box::new(GeneratorNode { value: 0.75 }));
        let plan = graph.compile().unwrap();

        let mut proc = GraphProcessor::new(2, 44100, 256);
        let handle = proc.swap_handle();
        handle.swap(plan);

        let output = proc.process();
        assert!(output.is_some());
        let buf = output.unwrap();
        assert!(buf.samples.iter().all(|&s| (s - 0.75).abs() < f32::EPSILON));
    }

    #[test]
    fn graph_processor_swap_plan() {
        let mut proc = GraphProcessor::new(1, 44100, 128);
        let handle = proc.swap_handle();

        // First plan: generate 0.5
        let mut g1 = Graph::new();
        let id1 = NodeId::next();
        g1.add_node(id1, Box::new(GeneratorNode { value: 0.5 }));
        handle.swap(g1.compile().unwrap());
        let out1 = proc.process().unwrap().samples[0];
        assert!((out1 - 0.5).abs() < f32::EPSILON);

        // Swap to new plan: generate 0.9
        let mut g2 = Graph::new();
        let id2 = NodeId::next();
        g2.add_node(id2, Box::new(GeneratorNode { value: 0.9 }));
        handle.swap(g2.compile().unwrap());
        let out2 = proc.process().unwrap().samples[0];
        assert!((out2 - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn graph_node_count() {
        let mut graph = Graph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.connection_count(), 0);
        let a = NodeId::next();
        let b = NodeId::next();
        graph.add_node(a, Box::new(PassthroughNode));
        graph.add_node(b, Box::new(GainNode { gain: 1.0 }));
        graph.connect(a, b);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.connection_count(), 1);
    }

    #[test]
    fn execution_plan_is_finished() {
        struct FinishedNode;
        impl AudioNode for FinishedNode {
            fn name(&self) -> &str { "finished" }
            fn num_inputs(&self) -> usize { 0 }
            fn num_outputs(&self) -> usize { 1 }
            fn process(&mut self, _inputs: &[&AudioBuffer], _output: &mut AudioBuffer) {}
            fn is_finished(&self) -> bool { true }
        }

        let mut graph = Graph::new();
        let id = NodeId::next();
        graph.add_node(id, Box::new(FinishedNode));
        let plan = graph.compile().unwrap();
        assert!(plan.is_finished());
    }

    #[test]
    fn graph_processor_is_finished() {
        struct FinishedNode;
        impl AudioNode for FinishedNode {
            fn name(&self) -> &str { "finished" }
            fn num_inputs(&self) -> usize { 0 }
            fn num_outputs(&self) -> usize { 1 }
            fn process(&mut self, _inputs: &[&AudioBuffer], _output: &mut AudioBuffer) {}
            fn is_finished(&self) -> bool { true }
        }

        let mut graph = Graph::new();
        let id = NodeId::next();
        graph.add_node(id, Box::new(FinishedNode));
        let plan = graph.compile().unwrap();

        let mut proc = GraphProcessor::new(1, 44100, 128);
        assert!(!proc.is_finished());
        let handle = proc.swap_handle();
        handle.swap(plan);
        proc.process();
        assert!(proc.is_finished());
    }

    #[test]
    fn linear_chain_processes_correctly() {
        let mut graph = Graph::new();
        let src = NodeId::next();
        let gain_node = NodeId::next();

        graph.add_node(src, Box::new(GeneratorNode { value: 1.0 }));
        graph.add_node(gain_node, Box::new(GainNode { gain: 0.5 }));
        graph.connect(src, gain_node);

        let plan = graph.compile().unwrap();
        let mut proc = GraphProcessor::new(1, 44100, 64);
        let handle = proc.swap_handle();
        handle.swap(plan);

        let output = proc.process().unwrap();
        // Generator outputs 1.0, gain multiplies by 0.5
        assert!(output.samples.iter().all(|&s| (s - 0.5).abs() < f32::EPSILON));
    }

    #[test]
    fn default_graph() {
        let graph = Graph::default();
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn swap_handle_clone() {
        let proc = GraphProcessor::new(1, 44100, 128);
        let handle1 = proc.swap_handle();
        let _handle2 = handle1.clone();
    }
}
