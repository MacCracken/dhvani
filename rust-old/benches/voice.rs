use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use dhvani::voice_synth::*;

fn bench_render_phoneme(c: &mut Criterion) {
    let voice = VoiceProfile::new_male();
    c.bench_function("render_phoneme_vowel_a_200ms", |b| {
        b.iter(|| {
            dhvani::voice_synth::render_phoneme(
                black_box(&Phoneme::VowelA),
                black_box(&voice),
                44100,
                0.2,
            )
            .unwrap()
        });
    });
}

fn bench_render_sequence(c: &mut Criterion) {
    let voice = VoiceProfile::new_female();
    let mut seq = PhonemeSequence::new();
    seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.15, Stress::Primary));
    seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.08, Stress::Unstressed));
    seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.15, Stress::Secondary));
    seq.push(PhonemeEvent::new(
        Phoneme::FricativeS,
        0.1,
        Stress::Unstressed,
    ));
    seq.push(PhonemeEvent::new(Phoneme::VowelE, 0.15, Stress::Primary));

    c.bench_function("render_sequence_5_phonemes", |b| {
        b.iter(|| {
            dhvani::voice_synth::render_sequence(black_box(&seq), black_box(&voice), 44100).unwrap()
        });
    });
}

fn bench_synthesis_pool(c: &mut Criterion) {
    let voice = VoiceProfile::new_male();

    c.bench_function("synthesis_pool_render_vowel_100ms", |b| {
        let mut pool = SynthesisPool::new(&voice, 44100.0).unwrap();
        b.iter(|| {
            let samples = pool
                .render(black_box(&Phoneme::VowelA), black_box(&voice), 0.1)
                .unwrap();
            black_box(samples.len());
        });
    });
}

fn bench_synthesis_pool_batch(c: &mut Criterion) {
    let voice = VoiceProfile::new_male();
    let phonemes: Vec<(&Phoneme, f32)> = vec![
        (&Phoneme::VowelA, 0.08),
        (&Phoneme::NasalN, 0.05),
        (&Phoneme::VowelI, 0.08),
        (&Phoneme::FricativeS, 0.06),
        (&Phoneme::VowelE, 0.08),
    ];

    c.bench_function("synthesis_pool_batch_5_phonemes", |b| {
        let mut pool = SynthesisPool::new(&voice, 44100.0).unwrap();
        b.iter(|| {
            pool.render_batch(black_box(&phonemes), black_box(&voice))
                .unwrap()
        });
    });
}

fn bench_batch_renderer(c: &mut Criterion) {
    let voice = VoiceProfile::new_male();

    c.bench_function("batch_renderer_5_phonemes", |b| {
        b.iter(|| {
            let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
            renderer.push(Phoneme::VowelA, 0.1, Stress::Primary);
            renderer.push(Phoneme::NasalN, 0.06, Stress::Unstressed);
            renderer.push(Phoneme::VowelI, 0.1, Stress::Primary);
            renderer.push(Phoneme::FricativeS, 0.06, Stress::Unstressed);
            renderer.push(Phoneme::VowelE, 0.1, Stress::Primary);
            renderer.render_all().unwrap()
        });
    });
}

fn bench_trajectory_planner(c: &mut Criterion) {
    let voice = VoiceProfile::new_male();
    let phonemes = [
        Phoneme::VowelA,
        Phoneme::NasalN,
        Phoneme::VowelI,
        Phoneme::FricativeS,
        Phoneme::VowelE,
    ];
    let durations = [0.1, 0.06, 0.1, 0.06, 0.1];

    c.bench_function("trajectory_plan_5_phonemes", |b| {
        b.iter(|| {
            TrajectoryPlanner::plan(
                black_box(&phonemes),
                black_box(&durations),
                black_box(&voice),
                44100.0,
            )
        });
    });

    let plan = TrajectoryPlanner::plan(&phonemes, &durations, &voice, 44100.0);
    let total = plan.total_samples();

    c.bench_function("trajectory_formants_at_1000_samples", |b| {
        b.iter(|| {
            for i in (0..total).step_by(total / 1000 + 1) {
                black_box(plan.formants_at(i));
            }
        });
    });
}

fn bench_detect_nasalization(c: &mut Criterion) {
    let phonemes = [
        Phoneme::VowelA,
        Phoneme::NasalN,
        Phoneme::VowelI,
        Phoneme::NasalM,
        Phoneme::VowelE,
        Phoneme::NasalN,
        Phoneme::VowelSchwa,
        Phoneme::PlosiveD,
        Phoneme::VowelO,
        Phoneme::NasalN,
    ];

    c.bench_function("detect_nasalization_10_phonemes", |b| {
        b.iter(|| detect_nasalization(black_box(&phonemes)));
    });
}

fn bench_glottal_vocal_tract(c: &mut Criterion) {
    let voice = VoiceProfile::new_male();
    let mut source = voice.create_glottal_source(44100.0).unwrap();
    let mut tract = VocalTract::new(44100.0);

    c.bench_function("glottal_vocal_tract_4410_frames", |b| {
        b.iter(|| {
            dhvani::voice_synth::render_vocal_tract(
                black_box(&mut source),
                black_box(&mut tract),
                4410,
                44100,
            )
        });
    });
}

criterion_group!(
    benches,
    bench_render_phoneme,
    bench_render_sequence,
    bench_synthesis_pool,
    bench_synthesis_pool_batch,
    bench_batch_renderer,
    bench_trajectory_planner,
    bench_detect_nasalization,
    bench_glottal_vocal_tract,
);
criterion_main!(benches);
