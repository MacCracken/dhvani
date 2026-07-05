#![no_main]
use libfuzzer_sys::fuzz_target;
use dhvani::buffer::AudioBuffer;
use dhvani::graph::{AudioNode, Graph, GraphProcessor, NodeId};

struct FuzzNode { value: f32 }
impl AudioNode for FuzzNode {
    fn name(&self) -> &str { "fuzz" }
    fn num_inputs(&self) -> usize { 0 }
    fn num_outputs(&self) -> usize { 1 }
    fn process(&mut self, _inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
        for s in &mut output.samples { *s = self.value; }
    }
}

struct PassNode;
impl AudioNode for PassNode {
    fn name(&self) -> &str { "pass" }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { 1 }
    fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
        if let Some(inp) = inputs.first() {
            output.samples.copy_from_slice(&inp.samples);
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 { return; }

    let node_count = (data[0] % 8).max(1) as usize;
    let mut graph = Graph::new();
    let mut ids = Vec::new();

    // First node is always a generator
    let first_id = NodeId::next();
    graph.add_node(first_id, Box::new(FuzzNode { value: 0.5 }));
    ids.push(first_id);

    // Remaining are passthroughs
    for _ in 1..node_count {
        let id = NodeId::next();
        graph.add_node(id, Box::new(PassNode));
        ids.push(id);
    }

    // Connect in chain (no cycles)
    for i in 1..ids.len() {
        graph.connect(ids[i - 1], ids[i]);
    }

    if let Ok(plan) = graph.compile() {
        let mut proc = GraphProcessor::new(1, 44100, 64);
        let handle = proc.swap_handle();
        handle.swap(plan);
        let _ = proc.process();
    }
});
