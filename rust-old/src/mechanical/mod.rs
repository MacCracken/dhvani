//! Mechanical sound synthesis — engines, gears, motors, turbines, RPM-driven harmonics.
//!
//! Re-exports from [`ghurni`](https://crates.io/crates/ghurni), the AGNOS mechanical sound crate.
//! Produces procedural, real-time mechanical and machine sounds driven by RPM parameters.
//!
//! # Feature: `mechanical`
//!
//! Enable with:
//! ```toml
//! dhvani = { version = "1", features = ["mechanical"] }
//! ```
//!
//! # Architecture
//!
//! ```text
//! RPM / load parameters → Engine/Motor/Gear → MechanicalMixer → AudioBuffer
//! ```

use crate::buffer::AudioBuffer;

// ── Engine ─────────────────────────────────────────────────────────

/// Internal combustion engine synthesis.
pub use ghurni::engine::{Engine, EngineType};

// ── Motor ──────────────────────────────────────────────────────────

/// Electric motor synthesis.
pub use ghurni::motor::{Motor, MotorType};

// ── Drivetrain ─────────────────────────────────────────────────────

/// Gear meshing synthesis.
pub use ghurni::gear::{Gear, GearMaterial};

/// Transmission synthesis.
pub use ghurni::transmission::Transmission;

/// Belt drive synthesis.
pub use ghurni::belt_drive::BeltDrive;

/// Chain drive synthesis.
pub use ghurni::chain_drive::ChainDrive;

/// Differential synthesis.
pub use ghurni::differential::Differential;

// ── Turbo / supercharger ───────────────────────────────────────────

/// Forced induction (turbo, supercharger) synthesis.
pub use ghurni::forced_induction::{ForcedInduction, InductionType};

/// Turbine synthesis.
pub use ghurni::turbine::Turbine;

// ── Timing ─────────────────────────────────────────────────────────

/// Mechanical clock/ticking synthesis.
pub use ghurni::clock::{Clock, ClockType};

// ── Mixing & events ────────────────────────────────────────────────

/// Multi-source mechanical sound mixer.
pub use ghurni::mixer::MechanicalMixer;

/// Mechanical event triggers (backfire, gear change, etc.).
pub use ghurni::event::MechanicalEvent;

// ── Presets ────────────────────────────────────────────────────────

/// Built-in mechanical sound presets.
pub use ghurni::presets;

// ── Traits ─────────────────────────────────────────────────────────

/// Common trait for RPM-driven mechanical synthesizers.
pub use ghurni::traits::Synthesizer;

// ── Error ──────────────────────────────────────────────────────────

/// Ghurni error type.
pub use ghurni::error::GhurniError;

// ── Bridge: ghurni synthesis → dhvani AudioBuffer ──────────────────

/// Render any [`Synthesizer`] to a dhvani [`AudioBuffer`] at the current RPM.
///
/// Calls `process_block` to fill a buffer of `frames` samples.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::mechanical::*;
///
/// let mut engine = Engine::new(EngineType::Gasoline, 4, 44100.0).unwrap();
/// engine.set_rpm(3000.0);
/// let buf = render_mechanical(&mut engine, 44100, 44100);
/// ```
pub fn render_mechanical(
    synth: &mut impl Synthesizer,
    frames: usize,
    sample_rate: u32,
) -> AudioBuffer {
    let mut samples = vec![0.0_f32; frames];
    synth.process_block(&mut samples);
    AudioBuffer::from_interleaved(samples, 1, sample_rate)
        .unwrap_or_else(|_| AudioBuffer::silence(1, frames, sample_rate.max(1)))
}

/// Render a [`Synthesizer`] at a specific RPM to a dhvani [`AudioBuffer`].
///
/// Sets RPM before rendering.
pub fn render_mechanical_at_rpm(
    synth: &mut impl Synthesizer,
    rpm: f32,
    frames: usize,
    sample_rate: u32,
) -> AudioBuffer {
    synth.set_rpm(rpm);
    render_mechanical(synth, frames, sample_rate)
}

/// Render an engine with specific RPM and load to a dhvani [`AudioBuffer`].
///
/// # Errors
///
/// Returns `crate::NadaError::Dsp` if synthesis fails.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::mechanical::*;
///
/// let mut engine = Engine::new(EngineType::Gasoline, 4, 44100.0).unwrap();
/// let buf = render_engine(&mut engine, 3500.0, 0.7, 44100, 1.0).unwrap();
/// ```
pub fn render_engine(
    engine: &mut Engine,
    rpm: f32,
    load: f32,
    sample_rate: u32,
    duration: f32,
) -> crate::Result<AudioBuffer> {
    let samples = engine
        .synthesize(rpm, load, duration)
        .map_err(|e| crate::NadaError::Dsp(format!("engine synthesis failed: {e}")))?;
    AudioBuffer::from_interleaved(samples, 1, sample_rate)
        .map_err(|e| crate::NadaError::Dsp(format!("buffer from engine output: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_gasoline_renders() {
        let mut engine = Engine::new(EngineType::Gasoline, 4, 44100.0).unwrap();
        let buf = render_engine(&mut engine, 3000.0, 0.5, 44100, 0.5).unwrap();
        assert!(buf.frames() > 0);
        assert!(buf.rms() > 0.0);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn motor_renders() {
        let mut motor = Motor::new(MotorType::Brushless, 6, 44100.0).unwrap();
        let buf = render_mechanical_at_rpm(&mut motor, 5000.0, 22050, 44100);
        assert_eq!(buf.frames(), 22050);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn gear_renders() {
        let mut gear = Gear::new(20, GearMaterial::Steel, 44100.0).unwrap();
        let buf = render_mechanical_at_rpm(&mut gear, 1200.0, 4410, 44100);
        assert!(buf.frames() > 0);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn turbine_renders() {
        let mut turbine = Turbine::new(12, 800.0, 44100.0).unwrap();
        let buf = render_mechanical_at_rpm(&mut turbine, 10000.0, 4410, 44100);
        assert!(buf.frames() > 0);
    }

    #[test]
    fn synthesizer_trait_rpm() {
        let mut motor = Motor::new(MotorType::Brushless, 6, 44100.0).unwrap();
        motor.set_rpm(2500.0);
        assert!((motor.rpm() - 2500.0).abs() < f32::EPSILON);
    }
}
