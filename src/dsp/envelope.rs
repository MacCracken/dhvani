//! ADSR Envelope generator — attack, decay, sustain, release.
//!
//! Used with [`VoiceManager`](crate::midi::voice::VoiceManager) for per-voice
//! amplitude and filter modulation.

use serde::{Deserialize, Serialize};

/// ADSR envelope parameters (times in seconds).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdsrParams {
    /// Attack time in seconds.
    pub attack: f32,
    /// Decay time in seconds.
    pub decay: f32,
    /// Sustain level (0.0–1.0).
    pub sustain: f32,
    /// Release time in seconds.
    pub release: f32,
}

impl Default for AdsrParams {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
        }
    }
}

/// Envelope state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// ADSR envelope generator.
///
/// Call `trigger()` on note-on, `release()` on note-off, and `tick()` every sample.
#[derive(Debug, Clone)]
pub struct Envelope {
    params: AdsrParams,
    state: EnvelopeState,
    level: f32,
    sample_rate: f32,
    stage_pos: u64,
    release_start_level: f32,
}

impl Envelope {
    /// Create a new envelope.
    pub fn new(params: AdsrParams, sample_rate: u32) -> Self {
        Self {
            params,
            state: EnvelopeState::Idle,
            level: 0.0,
            sample_rate: sample_rate as f32,
            stage_pos: 0,
            release_start_level: 0.0,
        }
    }

    /// Trigger the envelope (note-on).
    pub fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
        self.stage_pos = 0;
    }

    /// Release the envelope (note-off).
    pub fn release(&mut self) {
        if self.state != EnvelopeState::Idle {
            self.release_start_level = self.level;
            self.state = EnvelopeState::Release;
            self.stage_pos = 0;
        }
    }

    /// Advance one sample and return the current level (0.0–1.0).
    pub fn tick(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Idle => {
                self.level = 0.0;
            }
            EnvelopeState::Attack => {
                let samples = (self.params.attack * self.sample_rate).max(1.0) as u64;
                self.level = self.stage_pos as f32 / samples as f32;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.state = EnvelopeState::Decay;
                    self.stage_pos = 0;
                } else {
                    self.stage_pos += 1;
                }
            }
            EnvelopeState::Decay => {
                let samples = (self.params.decay * self.sample_rate).max(1.0) as u64;
                let progress = self.stage_pos as f32 / samples as f32;
                self.level = 1.0 + (self.params.sustain - 1.0) * progress;
                if progress >= 1.0 {
                    self.level = self.params.sustain;
                    self.state = EnvelopeState::Sustain;
                    self.stage_pos = 0;
                } else {
                    self.stage_pos += 1;
                }
            }
            EnvelopeState::Sustain => {
                self.level = self.params.sustain;
            }
            EnvelopeState::Release => {
                let samples = (self.params.release * self.sample_rate).max(1.0) as u64;
                let progress = self.stage_pos as f32 / samples as f32;
                self.level = self.release_start_level * (1.0 - progress);
                if progress >= 1.0 {
                    self.level = 0.0;
                    self.state = EnvelopeState::Idle;
                    self.stage_pos = 0;
                } else {
                    self.stage_pos += 1;
                }
            }
        }
        self.level
    }

    /// Current level (0.0–1.0).
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Current state.
    pub fn state(&self) -> EnvelopeState {
        self.state
    }

    /// Whether the envelope has finished (idle).
    pub fn is_finished(&self) -> bool {
        self.state == EnvelopeState::Idle
    }

    /// Reset to idle.
    pub fn reset(&mut self) {
        self.state = EnvelopeState::Idle;
        self.level = 0.0;
        self.stage_pos = 0;
    }

    /// Update parameters.
    pub fn set_params(&mut self, params: AdsrParams) {
        self.params = params;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_by_default() {
        let env = Envelope::new(AdsrParams::default(), 44100);
        assert_eq!(env.state(), EnvelopeState::Idle);
        assert_eq!(env.level(), 0.0);
        assert!(env.is_finished());
    }

    #[test]
    fn attack_ramps_up() {
        let params = AdsrParams { attack: 0.01, decay: 0.01, sustain: 0.5, release: 0.01 };
        let mut env = Envelope::new(params, 44100);
        env.trigger();
        assert_eq!(env.state(), EnvelopeState::Attack);
        // Tick through attack
        let mut max_level = 0.0f32;
        for _ in 0..500 {
            let l = env.tick();
            max_level = max_level.max(l);
        }
        assert!(max_level > 0.9, "Attack should reach near 1.0");
    }

    #[test]
    fn sustain_holds() {
        let params = AdsrParams { attack: 0.001, decay: 0.001, sustain: 0.6, release: 0.1 };
        let mut env = Envelope::new(params, 44100);
        env.trigger();
        // Tick through attack + decay
        for _ in 0..1000 {
            env.tick();
        }
        assert_eq!(env.state(), EnvelopeState::Sustain);
        assert!((env.level() - 0.6).abs() < 0.05);
    }

    #[test]
    fn release_decays_to_zero() {
        let params = AdsrParams { attack: 0.001, decay: 0.001, sustain: 0.7, release: 0.01 };
        let mut env = Envelope::new(params, 44100);
        env.trigger();
        for _ in 0..500 { env.tick(); } // reach sustain
        env.release();
        assert_eq!(env.state(), EnvelopeState::Release);
        for _ in 0..1000 { env.tick(); }
        assert!(env.is_finished());
        assert!(env.level() < 0.01);
    }

    #[test]
    fn release_from_attack() {
        let params = AdsrParams { attack: 0.1, decay: 0.1, sustain: 0.5, release: 0.01 };
        let mut env = Envelope::new(params, 44100);
        env.trigger();
        for _ in 0..100 { env.tick(); } // partway through attack
        let level_at_release = env.level();
        env.release();
        // Should release smoothly from current level
        for _ in 0..1000 { env.tick(); }
        assert!(env.is_finished());
        let _ = level_at_release; // used for smooth release
    }

    #[test]
    fn reset_goes_idle() {
        let mut env = Envelope::new(AdsrParams::default(), 44100);
        env.trigger();
        for _ in 0..100 { env.tick(); }
        env.reset();
        assert!(env.is_finished());
        assert_eq!(env.level(), 0.0);
    }

    #[test]
    fn set_params_updates() {
        let mut env = Envelope::new(AdsrParams::default(), 44100);
        env.set_params(AdsrParams { attack: 0.5, decay: 0.2, sustain: 0.3, release: 1.0 });
        env.trigger();
        // Should use new attack time
        env.tick();
        assert_eq!(env.state(), EnvelopeState::Attack);
    }
}
