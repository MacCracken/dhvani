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
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) u32);

impl NodeId {
    /// Generate the next unique node ID.
    pub fn next() -> Self {
        Self(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// The raw node ID value.
    pub fn value(self) -> u32 {
        self.0
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
    /// Whether this node is currently bypassed.
    ///
    /// When bypassed, the graph processor passes the first input directly
    /// to the output without calling `process()`.
    fn is_bypassed(&self) -> bool {
        false
    }
    /// Set the bypass state. Returns `false` if the node doesn't support bypass.
    fn set_bypass(&mut self, _bypassed: bool) -> bool {
        false
    }
    /// Latency introduced by this node, in frames.
    ///
    /// Used by the graph processor for latency compensation across parallel paths.
    /// Default is 0 (no latency).
    fn latency_frames(&self) -> usize {
        0
    }
}

// ── Connection ──────────────────────────────────────────────────────

/// A directed connection from one node's output to another's input.
#[must_use]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct Connection {
    /// Source node.
    pub(crate) from: NodeId,
    /// Destination node.
    pub(crate) to: NodeId,
}

impl Connection {
    /// Source node.
    pub fn from(&self) -> NodeId {
        self.from
    }

    /// Destination node.
    pub fn to(&self) -> NodeId {
        self.to
    }
}

// ── Graph (non-RT builder) ──────────────────────────────────────────

/// Non-real-time audio graph builder.
///
/// Add nodes and connections, then `compile()` to produce an `ExecutionPlan`.
#[must_use]
pub struct Graph {
    nodes: HashMap<NodeId, Box<dyn AudioNode>>,
    connections: Vec<Connection>,
}

impl Graph {
    /// Create an empty audio graph.
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
        tracing::debug!(
            nodes = self.nodes.len(),
            connections = self.connections.len(),
            "Graph::compile: started"
        );
        let order = topological_sort(&self.nodes, &self.connections)?;
        let mut input_map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for conn in &self.connections {
            input_map.entry(conn.to).or_default().push(conn.from);
        }

        // Compute latency compensation: for nodes with multiple inputs,
        // shorter paths need delay to align with the longest path.
        let mut path_latency: HashMap<NodeId, usize> = HashMap::new();
        for &id in &order {
            let own = self.nodes.get(&id).map(|n| n.latency_frames()).unwrap_or(0);
            let input_max = input_map
                .get(&id)
                .map(|inputs| {
                    inputs
                        .iter()
                        .filter_map(|inp| path_latency.get(inp))
                        .copied()
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            path_latency.insert(id, input_max + own);
        }

        // For each node, compute how much compensation delay its inputs need
        let mut latency_comp: HashMap<NodeId, usize> = HashMap::new();
        for &id in &order {
            if let Some(inputs) = input_map.get(&id)
                && inputs.len() > 1
            {
                let max_input_latency = inputs
                    .iter()
                    .filter_map(|inp| path_latency.get(inp))
                    .copied()
                    .max()
                    .unwrap_or(0);
                for inp in inputs {
                    let inp_lat = path_latency.get(inp).copied().unwrap_or(0);
                    let comp = max_input_latency - inp_lat;
                    if comp > 0 {
                        latency_comp.insert(*inp, comp);
                    }
                }
            }
        }

        // Compute level-grouped execution order for parallel processing.
        // Nodes at the same level have no dependencies on each other.
        let levels = compute_levels(&order, &input_map);

        Ok(ExecutionPlan {
            order,
            levels,
            nodes: self.nodes,
            input_map,
            latency_comp,
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
#[must_use]
pub struct ExecutionPlan {
    order: Vec<NodeId>,
    /// Level-grouped execution order: nodes within a level are independent and can run in parallel.
    levels: Vec<Vec<NodeId>>,
    nodes: HashMap<NodeId, Box<dyn AudioNode>>,
    input_map: HashMap<NodeId, Vec<NodeId>>,
    /// Per-node latency compensation delay (frames to add before this node's output).
    latency_comp: HashMap<NodeId, usize>,
}

impl ExecutionPlan {
    /// Execution order (topologically sorted node IDs).
    pub fn order(&self) -> &[NodeId] {
        &self.order
    }

    /// Level-grouped execution order. Nodes within each level are independent.
    pub fn levels(&self) -> &[Vec<NodeId>] {
        &self.levels
    }

    /// Check if the last node is finished.
    pub fn is_finished(&self) -> bool {
        self.order
            .last()
            .and_then(|id| self.nodes.get(id))
            .is_some_and(|n| n.is_finished())
    }

    /// Set bypass state for a node. Returns `false` if the node doesn't exist or doesn't support bypass.
    pub fn set_bypass(&mut self, id: NodeId, bypassed: bool) -> bool {
        self.nodes
            .get_mut(&id)
            .is_some_and(|n| n.set_bypass(bypassed))
    }

    /// Query bypass state for a node.
    pub fn is_bypassed(&self, id: NodeId) -> bool {
        self.nodes.get(&id).is_some_and(|n| n.is_bypassed())
    }

    /// Query latency for a node (frames).
    pub fn latency_frames(&self, id: NodeId) -> usize {
        self.nodes.get(&id).map(|n| n.latency_frames()).unwrap_or(0)
    }

    /// Latency compensation delay for a node's output (frames).
    ///
    /// In parallel paths, shorter paths need extra delay to align with longer ones.
    /// Returns 0 if no compensation is needed.
    #[must_use]
    pub fn compensation_delay(&self, id: NodeId) -> usize {
        self.latency_comp.get(&id).copied().unwrap_or(0)
    }

    /// Total pipeline latency (maximum path latency from source to sink).
    #[must_use]
    pub fn total_latency(&self) -> usize {
        // Compute max accumulated latency through the graph
        let mut node_latency: HashMap<NodeId, usize> = HashMap::new();
        for &id in &self.order {
            let own = self.nodes.get(&id).map(|n| n.latency_frames()).unwrap_or(0);
            let input_max = self
                .input_map
                .get(&id)
                .map(|inputs| {
                    inputs
                        .iter()
                        .filter_map(|inp| node_latency.get(inp))
                        .copied()
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            node_latency.insert(id, input_max + own);
        }
        node_latency.values().copied().max().unwrap_or(0)
    }
}

// ── GraphProcessor (RT-thread) ──────────────────────────────────────

/// Real-time audio graph processor with double-buffered plan swapping.
///
/// The RT thread calls `process()` each buffer cycle. New plans are swapped
/// in from the non-RT thread via `GraphSwapHandle` without blocking.
#[must_use]
pub struct GraphProcessor {
    current_plan: Option<ExecutionPlan>,
    pending_plan: Arc<Mutex<Option<ExecutionPlan>>>,
    node_outputs: Vec<Option<AudioBuffer>>,
    /// Pre-allocated scratch for gathering input buffers.
    input_scratch: Vec<AudioBuffer>,
    channels: u32,
    sample_rate: u32,
    buffer_frames: usize,
}

impl GraphProcessor {
    /// Create a new graph processor with the given audio format.
    pub fn new(channels: u32, sample_rate: u32, buffer_frames: usize) -> Self {
        Self {
            current_plan: None,
            pending_plan: Arc::new(Mutex::new(None)),
            node_outputs: Vec::new(),
            input_scratch: Vec::new(),
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
            tracing::debug!(
                nodes = new_plan.order.len(),
                "GraphProcessor: swapped to new plan"
            );
            // Pre-allocate output slots based on max node ID
            let max_id = new_plan
                .order
                .iter()
                .map(|id| id.0 as usize)
                .max()
                .unwrap_or(0);
            self.node_outputs.clear();
            self.node_outputs.resize_with(max_id + 1, || None);
            self.current_plan = Some(new_plan);
        }

        let plan = self.current_plan.as_mut()?;

        // Process nodes in topological order
        for i in 0..plan.order.len() {
            let node_id = plan.order[i];
            let idx = node_id.0 as usize;

            // Gather input buffers into scratch (avoids borrow conflict with node_outputs)
            self.input_scratch.clear();
            if let Some(ids) = plan.input_map.get(&node_id) {
                for id in ids {
                    if let Some(Some(buf)) = self.node_outputs.get(id.0 as usize) {
                        self.input_scratch.push(buf.clone());
                    }
                }
            }

            let input_refs: Vec<&AudioBuffer> = self.input_scratch.iter().collect();

            // Take output buffer from slot (reuse allocation) or create new
            if idx >= self.node_outputs.len() {
                self.node_outputs.resize_with(idx + 1, || None);
            }
            let mut output = self.node_outputs[idx].take().unwrap_or_else(|| {
                AudioBuffer::silence(self.channels, self.buffer_frames, self.sample_rate)
            });
            output.samples_mut().fill(0.0);

            if let Some(node) = plan.nodes.get_mut(&node_id) {
                if node.is_bypassed() {
                    // Bypass: pass first input directly to output
                    if let Some(first) = input_refs.first() {
                        output.samples_mut().copy_from_slice(first.samples());
                    }
                } else {
                    node.process(&input_refs, &mut output);
                }
            }

            self.node_outputs[idx] = Some(output);
        }

        // Return the last node's output
        plan.order
            .last()
            .and_then(|id| self.node_outputs.get(id.0 as usize))
            .and_then(|opt: &Option<AudioBuffer>| opt.as_ref())
    }

    /// Process using level-parallel execution (requires `parallel` feature).
    ///
    /// Nodes at the same level in the DAG are processed in parallel using rayon.
    /// Falls back to sequential processing for levels with a single node.
    ///
    /// Returns a reference to the output buffer, or None if no plan is active.
    #[cfg(feature = "parallel")]
    pub fn process_parallel(&mut self) -> Option<&AudioBuffer> {
        use rayon::prelude::*;

        // Try to pick up pending plan (same as sequential)
        if let Ok(mut pending) = self.pending_plan.try_lock()
            && let Some(new_plan) = pending.take()
        {
            tracing::debug!(
                nodes = new_plan.order.len(),
                levels = new_plan.levels.len(),
                "GraphProcessor: swapped to new plan (parallel)"
            );
            let max_id = new_plan
                .order
                .iter()
                .map(|id| id.0 as usize)
                .max()
                .unwrap_or(0);
            self.node_outputs.clear();
            self.node_outputs.resize_with(max_id + 1, || None);
            self.current_plan = Some(new_plan);
        }

        let plan = self.current_plan.as_mut()?;

        // Process levels sequentially; nodes within each level in parallel
        for level_idx in 0..plan.levels.len() {
            let level = &plan.levels[level_idx];

            if level.len() == 1 {
                // Single node — process sequentially (no rayon overhead)
                let node_id = level[0];
                let idx = node_id.0 as usize;

                self.input_scratch.clear();
                if let Some(ids) = plan.input_map.get(&node_id) {
                    for id in ids {
                        if let Some(Some(buf)) = self.node_outputs.get(id.0 as usize) {
                            self.input_scratch.push(buf.clone());
                        }
                    }
                }
                let input_refs: Vec<&AudioBuffer> = self.input_scratch.iter().collect();

                if idx >= self.node_outputs.len() {
                    self.node_outputs.resize_with(idx + 1, || None);
                }
                let mut output = self.node_outputs[idx].take().unwrap_or_else(|| {
                    AudioBuffer::silence(self.channels, self.buffer_frames, self.sample_rate)
                });
                output.samples_mut().fill(0.0);

                if let Some(node) = plan.nodes.get_mut(&node_id) {
                    if node.is_bypassed() {
                        if let Some(first) = input_refs.first() {
                            output.samples_mut().copy_from_slice(first.samples());
                        }
                    } else {
                        node.process(&input_refs, &mut output);
                    }
                }
                self.node_outputs[idx] = Some(output);
            } else {
                // Multiple independent nodes — temporarily extract nodes for parallel processing
                #[allow(clippy::type_complexity)]
                let mut work: Vec<(
                    NodeId,
                    Box<dyn AudioNode>,
                    Vec<AudioBuffer>,
                    AudioBuffer,
                )> = Vec::with_capacity(level.len());

                for &node_id in level {
                    let idx = node_id.0 as usize;

                    let mut inputs = Vec::new();
                    if let Some(ids) = plan.input_map.get(&node_id) {
                        for id in ids {
                            if let Some(Some(buf)) = self.node_outputs.get(id.0 as usize) {
                                inputs.push(buf.clone());
                            }
                        }
                    }

                    if idx >= self.node_outputs.len() {
                        self.node_outputs.resize_with(idx + 1, || None);
                    }
                    let mut output = self.node_outputs[idx].take().unwrap_or_else(|| {
                        AudioBuffer::silence(self.channels, self.buffer_frames, self.sample_rate)
                    });
                    output.samples_mut().fill(0.0);

                    // Take node out of map for parallel ownership
                    if let Some(node) = plan.nodes.remove(&node_id) {
                        work.push((node_id, node, inputs, output));
                    }
                }

                // Process in parallel — each item owns its node
                work.par_iter_mut().for_each(|(_, node, inputs, output)| {
                    let input_refs: Vec<&AudioBuffer> = inputs.iter().collect();
                    if node.is_bypassed() {
                        if let Some(first) = input_refs.first() {
                            output.samples_mut().copy_from_slice(first.samples());
                        }
                    } else {
                        node.process(&input_refs, output);
                    }
                });

                // Put nodes and outputs back
                for (node_id, node, _, output) in work {
                    plan.nodes.insert(node_id, node);
                    self.node_outputs[node_id.0 as usize] = Some(output);
                }
            }
        }

        // Return the last node's output
        plan.order
            .last()
            .and_then(|id| self.node_outputs.get(id.0 as usize))
            .and_then(|opt: &Option<AudioBuffer>| opt.as_ref())
    }

    /// Whether the current plan's last node is finished.
    pub fn is_finished(&self) -> bool {
        self.current_plan.as_ref().is_some_and(|p| p.is_finished())
    }
}

/// Handle for the non-RT thread to swap in new execution plans.
#[must_use]
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

// ── Level computation ──────────────────────────────────────────────

/// Group topologically sorted nodes into levels based on dependency depth.
/// Nodes at the same level are independent and can be processed in parallel.
fn compute_levels(order: &[NodeId], input_map: &HashMap<NodeId, Vec<NodeId>>) -> Vec<Vec<NodeId>> {
    let mut depth: HashMap<NodeId, usize> = HashMap::new();

    for &id in order {
        let max_input_depth = input_map
            .get(&id)
            .map(|inputs| {
                inputs
                    .iter()
                    .filter_map(|inp| depth.get(inp))
                    .copied()
                    .max()
                    .map(|d| d + 1)
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        depth.insert(id, max_input_depth);
    }

    let max_depth = depth.values().copied().max().unwrap_or(0);
    let mut levels = vec![Vec::new(); max_depth + 1];
    for &id in order {
        if let Some(&d) = depth.get(&id) {
            levels[d].push(id);
        }
    }

    // Remove empty levels
    levels.retain(|l| !l.is_empty());
    levels
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
        fn name(&self) -> &str {
            "passthrough"
        }
        fn num_inputs(&self) -> usize {
            1
        }
        fn num_outputs(&self) -> usize {
            1
        }
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
        fn name(&self) -> &str {
            "generator"
        }
        fn num_inputs(&self) -> usize {
            0
        }
        fn num_outputs(&self) -> usize {
            1
        }
        fn process(&mut self, _inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
            for s in &mut output.samples {
                *s = self.value;
            }
        }
    }

    struct GainNode {
        gain: f32,
    }
    impl AudioNode for GainNode {
        fn name(&self) -> &str {
            "gain"
        }
        fn num_inputs(&self) -> usize {
            1
        }
        fn num_outputs(&self) -> usize {
            1
        }
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
            fn name(&self) -> &str {
                "finished"
            }
            fn num_inputs(&self) -> usize {
                0
            }
            fn num_outputs(&self) -> usize {
                1
            }
            fn process(&mut self, _inputs: &[&AudioBuffer], _output: &mut AudioBuffer) {}
            fn is_finished(&self) -> bool {
                true
            }
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
            fn name(&self) -> &str {
                "finished"
            }
            fn num_inputs(&self) -> usize {
                0
            }
            fn num_outputs(&self) -> usize {
                1
            }
            fn process(&mut self, _inputs: &[&AudioBuffer], _output: &mut AudioBuffer) {}
            fn is_finished(&self) -> bool {
                true
            }
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
        assert!(
            output
                .samples
                .iter()
                .all(|&s| (s - 0.5).abs() < f32::EPSILON)
        );
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

    // ── Bypass tests ───────────────────────────────────────────────

    struct BypassableGainNode {
        gain: f32,
        bypassed: bool,
    }
    impl AudioNode for BypassableGainNode {
        fn name(&self) -> &str {
            "bypassable_gain"
        }
        fn num_inputs(&self) -> usize {
            1
        }
        fn num_outputs(&self) -> usize {
            1
        }
        fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
            if let Some(input) = inputs.first() {
                for (i, s) in output.samples.iter_mut().enumerate() {
                    *s = input.samples.get(i).copied().unwrap_or(0.0) * self.gain;
                }
            }
        }
        fn is_bypassed(&self) -> bool {
            self.bypassed
        }
        fn set_bypass(&mut self, bypassed: bool) -> bool {
            self.bypassed = bypassed;
            true
        }
    }

    #[test]
    fn node_bypass_passes_input() {
        let mut graph = Graph::new();
        let src = NodeId::next();
        let gain_id = NodeId::next();

        graph.add_node(src, Box::new(GeneratorNode { value: 1.0 }));
        graph.add_node(
            gain_id,
            Box::new(BypassableGainNode {
                gain: 0.5,
                bypassed: true,
            }),
        );
        graph.connect(src, gain_id);

        let plan = graph.compile().unwrap();
        let mut proc = GraphProcessor::new(1, 44100, 64);
        proc.swap_handle().swap(plan);

        let output = proc.process().unwrap();
        // Bypassed gain node should pass input (1.0) unchanged, not multiply by 0.5
        assert!(
            output
                .samples
                .iter()
                .all(|&s| (s - 1.0).abs() < f32::EPSILON),
            "bypass didn't pass through: got {}",
            output.samples[0]
        );
    }

    #[test]
    fn node_bypass_toggle() {
        let mut graph = Graph::new();
        let src = NodeId::next();
        let gain_id = NodeId::next();

        graph.add_node(src, Box::new(GeneratorNode { value: 1.0 }));
        graph.add_node(
            gain_id,
            Box::new(BypassableGainNode {
                gain: 0.5,
                bypassed: false,
            }),
        );
        graph.connect(src, gain_id);

        let mut plan = graph.compile().unwrap();

        // Initially not bypassed
        assert!(!plan.is_bypassed(gain_id));

        // Enable bypass
        assert!(plan.set_bypass(gain_id, true));
        assert!(plan.is_bypassed(gain_id));

        // Disable bypass
        assert!(plan.set_bypass(gain_id, false));
        assert!(!plan.is_bypassed(gain_id));
    }

    // ── Latency tests ──────────────────────────────────────────────

    struct LatencyNode {
        latency: usize,
    }
    impl AudioNode for LatencyNode {
        fn name(&self) -> &str {
            "latency"
        }
        fn num_inputs(&self) -> usize {
            1
        }
        fn num_outputs(&self) -> usize {
            1
        }
        fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
            if let Some(input) = inputs.first() {
                output.samples.copy_from_slice(&input.samples);
            }
        }
        fn latency_frames(&self) -> usize {
            self.latency
        }
    }

    #[test]
    fn latency_single_node() {
        let mut graph = Graph::new();
        let id = NodeId::next();
        graph.add_node(id, Box::new(LatencyNode { latency: 256 }));
        let plan = graph.compile().unwrap();
        assert_eq!(plan.total_latency(), 256);
        assert_eq!(plan.latency_frames(id), 256);
    }

    #[test]
    fn latency_chain_accumulates() {
        let mut graph = Graph::new();
        let a = NodeId::next();
        let b = NodeId::next();
        let c = NodeId::next();

        graph.add_node(a, Box::new(LatencyNode { latency: 100 }));
        graph.add_node(b, Box::new(LatencyNode { latency: 200 }));
        graph.add_node(c, Box::new(LatencyNode { latency: 50 }));

        graph.connect(a, b);
        graph.connect(b, c);

        let plan = graph.compile().unwrap();
        // Total = 100 + 200 + 50 = 350
        assert_eq!(plan.total_latency(), 350);
    }

    #[test]
    fn levels_computed_correctly() {
        // Diamond graph: src → [gain_a, gain_b] → out
        let mut graph = Graph::new();
        let src = NodeId::next();
        let a = NodeId::next();
        let b = NodeId::next();
        let out = NodeId::next();

        graph.add_node(src, Box::new(GeneratorNode { value: 1.0 }));
        graph.add_node(a, Box::new(GainNode { gain: 0.5 }));
        graph.add_node(b, Box::new(GainNode { gain: 0.3 }));
        graph.add_node(out, Box::new(PassthroughNode));

        graph.connect(src, a);
        graph.connect(src, b);
        graph.connect(a, out);
        graph.connect(b, out);

        let plan = graph.compile().unwrap();
        let levels = plan.levels();

        // Level 0: src, Level 1: a + b (parallel), Level 2: out
        assert_eq!(levels.len(), 3, "expected 3 levels, got {}", levels.len());
        assert_eq!(levels[0].len(), 1, "level 0 should have 1 node (src)");
        assert_eq!(levels[1].len(), 2, "level 1 should have 2 nodes (a, b)");
        assert_eq!(levels[2].len(), 1, "level 2 should have 1 node (out)");
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_process_matches_sequential() {
        // Build a diamond graph and verify parallel produces same result as sequential
        let mut seq_graph = Graph::new();
        let src1 = NodeId::next();
        let ga = NodeId::next();
        let gb = NodeId::next();

        seq_graph.add_node(src1, Box::new(GeneratorNode { value: 1.0 }));
        seq_graph.add_node(ga, Box::new(GainNode { gain: 0.5 }));
        seq_graph.add_node(gb, Box::new(GainNode { gain: 0.3 }));

        seq_graph.connect(src1, ga);
        seq_graph.connect(src1, gb);

        let plan = seq_graph.compile().unwrap();
        let mut proc = GraphProcessor::new(1, 44100, 64);
        proc.swap_handle().swap(plan);

        // Sequential
        let seq_output = proc.process().unwrap().samples().to_vec();

        // Rebuild identical graph for parallel
        let mut par_graph = Graph::new();
        let src2 = NodeId::next();
        let gc = NodeId::next();
        let gd = NodeId::next();

        par_graph.add_node(src2, Box::new(GeneratorNode { value: 1.0 }));
        par_graph.add_node(gc, Box::new(GainNode { gain: 0.5 }));
        par_graph.add_node(gd, Box::new(GainNode { gain: 0.3 }));

        par_graph.connect(src2, gc);
        par_graph.connect(src2, gd);

        let plan2 = par_graph.compile().unwrap();
        let mut proc2 = GraphProcessor::new(1, 44100, 64);
        proc2.swap_handle().swap(plan2);

        let par_output = proc2.process_parallel().unwrap().samples().to_vec();

        // Both should produce the same last-node output
        // (last node in topo order processes identically)
        assert_eq!(seq_output.len(), par_output.len());
        for (s, p) in seq_output.iter().zip(par_output.iter()) {
            assert!((s - p).abs() < 1e-6, "parallel mismatch: seq={s} par={p}");
        }
    }

    #[test]
    fn latency_zero_by_default() {
        let mut graph = Graph::new();
        let id = NodeId::next();
        graph.add_node(id, Box::new(PassthroughNode));
        let plan = graph.compile().unwrap();
        assert_eq!(plan.total_latency(), 0);
        assert_eq!(plan.latency_frames(id), 0);
    }
}
