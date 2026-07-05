use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use dhvani::g2p::*;

fn bench_text_to_phonemes(c: &mut Criterion) {
    let engine = G2PEngine::new(Language::English);

    c.bench_function("g2p_hello_world", |b| {
        b.iter(|| text_to_phonemes(black_box(&engine), black_box("hello world")).unwrap());
    });
}

fn bench_text_to_phonemes_sentence(c: &mut Criterion) {
    let engine = G2PEngine::new(Language::English);
    let sentence = "The quick brown fox jumps over the lazy dog";

    c.bench_function("g2p_sentence_9_words", |b| {
        b.iter(|| text_to_phonemes(black_box(&engine), black_box(sentence)).unwrap());
    });
}

fn bench_speak(c: &mut Criterion) {
    let engine = G2PEngine::new(Language::English);
    let voice = svara::voice::VoiceProfile::new_female();

    c.bench_function("speak_hello", |b| {
        b.iter(|| {
            speak(
                black_box(&engine),
                black_box("hello"),
                black_box(&voice),
                44100,
            )
            .unwrap()
        });
    });
}

fn bench_heteronym_lookup(c: &mut Criterion) {
    c.bench_function("heteronym_lookup_read", |b| {
        b.iter(|| heteronym::lookup(black_box("read")));
    });

    c.bench_function("heteronym_lookup_miss", |b| {
        b.iter(|| heteronym::lookup(black_box("hello")));
    });
}

fn bench_heteronym_select_variant(c: &mut Criterion) {
    let rule = heteronym::lookup("read").unwrap();
    let context = ["want", "to"];

    c.bench_function("heteronym_select_variant", |b| {
        b.iter(|| heteronym::select_variant(black_box(rule), black_box(&context)));
    });
}

fn bench_ssml_parse(c: &mut Criterion) {
    let simple = "Hello world";
    let with_break = "Hello <break time=\"500ms\"/> world";
    let complex = "<speak><emphasis level=\"strong\">Important</emphasis> <break time=\"200ms\"/> <prosody rate=\"fast\">quick note</prosody></speak>";

    c.bench_function("ssml_parse_plain", |b| {
        b.iter(|| ssml::parse(black_box(simple)).unwrap());
    });

    c.bench_function("ssml_parse_with_break", |b| {
        b.iter(|| ssml::parse(black_box(with_break)).unwrap());
    });

    c.bench_function("ssml_parse_complex", |b| {
        b.iter(|| ssml::parse(black_box(complex)).unwrap());
    });
}

criterion_group!(
    benches,
    bench_text_to_phonemes,
    bench_text_to_phonemes_sentence,
    bench_speak,
    bench_heteronym_lookup,
    bench_heteronym_select_variant,
    bench_ssml_parse,
);
criterion_main!(benches);
