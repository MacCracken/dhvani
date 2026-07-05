//! Audio graph: build a processing graph, compile, and process.

use dhvani::buffer::AudioBuffer;
use dhvani::graph::{AudioNode, Graph, GraphProcessor, NodeId};

// A tone generator node
struct ToneGenerator {
    freq: f32,
    phase: f64,
    sample_rate: f64,
}

impl AudioNode for ToneGenerator {
    fn name(&self) -> &str {
        "tone"
    }
    fn num_inputs(&self) -> usize {
        0
    }
    fn num_outputs(&self) -> usize {
        1
    }
    fn process(&mut self, _inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
        for s in output.samples_mut() {
            *s = (self.phase as f32).sin() * 0.5;
            self.phase += 2.0 * std::f64::consts::PI * self.freq as f64 / self.sample_rate;
        }
    }
}

// A gain node
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
            for (i, s) in output.samples_mut().iter_mut().enumerate() {
                *s = input.samples().get(i).copied().unwrap_or(0.0) * self.gain;
            }
        }
    }
}

fn main() {
    let sr = 44100u32;
    let buffer_size = 1024;

    // Build graph: ToneGenerator → GainNode
    let mut graph = Graph::new();
    let tone_id = NodeId::next();
    let gain_id = NodeId::next();

    graph.add_node(
        tone_id,
        Box::new(ToneGenerator {
            freq: 440.0,
            phase: 0.0,
            sample_rate: sr as f64,
        }),
    );
    graph.add_node(gain_id, Box::new(GainNode { gain: 0.3 }));
    graph.connect(tone_id, gain_id);

    println!(
        "Graph: {} nodes, {} connections",
        graph.node_count(),
        graph.connection_count()
    );

    // Compile to execution plan
    let plan = graph.compile().unwrap();
    println!("Execution order: {} nodes", plan.order().len());

    // Create RT processor and swap in the plan
    let mut processor = GraphProcessor::new(1, sr, buffer_size);
    let handle = processor.swap_handle();
    handle.swap(plan);

    // Process a few buffers
    for i in 0..4 {
        if let Some(output) = processor.process() {
            println!(
                "Buffer {}: {} frames, peak={:.3}, rms={:.3}",
                i,
                output.frames(),
                output.peak(),
                output.rms()
            );
        }
    }
}
