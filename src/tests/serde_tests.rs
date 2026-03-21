//! Serde roundtrip tests for all serializable types.

use crate::buffer::SampleFormat;
use crate::clock::AudioClock;
use crate::dsp::biquad::{BiquadCoeffs, FilterType};
use crate::dsp::compressor::CompressorParams;
use crate::dsp::deesser::DeEsserParams;
use crate::dsp::delay::ModulatedDelayParams;
use crate::dsp::envelope::AdsrParams;
use crate::dsp::eq::{BandType, EqBandConfig};
use crate::dsp::lfo::LfoShape;
use crate::dsp::limiter::LimiterParams;
use crate::dsp::oscillator::Waveform;
use crate::dsp::reverb::ReverbParams;
use crate::midi::routing::{CcMapping, MidiRoute, VelocityCurve};
use crate::midi::v2::{ControlChangeV2, NoteOnV2, UmpMessageType};
use crate::midi::{ControlChange, MidiClip, MidiEvent, NoteEvent};
use crate::capture::{AudioDevice, CaptureConfig, DeviceType, OutputConfig};
use crate::capture::record::RecordingMode;

macro_rules! serde_roundtrip {
    ($name:ident, $value:expr, $type:ty) => {
        #[test]
        fn $name() {
            let original: $type = $value;
            let json = serde_json::to_string(&original).expect("serialize");
            let back: $type = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(
                serde_json::to_string(&original).unwrap(),
                serde_json::to_string(&back).unwrap()
            );
        }
    };
}

serde_roundtrip!(sample_format_f32, SampleFormat::F32, SampleFormat);
serde_roundtrip!(sample_format_i16, SampleFormat::I16, SampleFormat);

serde_roundtrip!(
    audio_clock,
    AudioClock::with_tempo(48000, 120.0),
    AudioClock
);

serde_roundtrip!(
    compressor_params,
    CompressorParams::default(),
    CompressorParams
);

serde_roundtrip!(
    reverb_params,
    ReverbParams { room_size: 0.6, damping: 0.4, mix: 0.3 },
    ReverbParams
);

serde_roundtrip!(limiter_params, LimiterParams::default(), LimiterParams);

serde_roundtrip!(deesser_params, DeEsserParams::default(), DeEsserParams);

serde_roundtrip!(
    modulated_delay_params,
    ModulatedDelayParams::default(),
    ModulatedDelayParams
);

serde_roundtrip!(adsr_params, AdsrParams::default(), AdsrParams);

serde_roundtrip!(
    eq_band_config,
    EqBandConfig {
        band_type: BandType::Peaking,
        freq_hz: 1000.0,
        gain_db: 3.0,
        q: 1.5,
        enabled: true,
    },
    EqBandConfig
);

serde_roundtrip!(filter_type_lp, FilterType::LowPass, FilterType);
serde_roundtrip!(
    filter_type_peaking,
    FilterType::Peaking { gain_db: 6.0 },
    FilterType
);

serde_roundtrip!(waveform_sine, Waveform::Sine, Waveform);
serde_roundtrip!(waveform_saw, Waveform::Saw, Waveform);
serde_roundtrip!(lfo_shape_sh, LfoShape::SampleAndHold, LfoShape);
serde_roundtrip!(band_type_notch, BandType::Notch, BandType);

serde_roundtrip!(
    note_event,
    NoteEvent { position: 100, duration: 500, note: 60, velocity: 100, channel: 0 },
    NoteEvent
);

serde_roundtrip!(
    control_change,
    ControlChange { position: 0, controller: 7, value: 100, channel: 0 },
    ControlChange
);

serde_roundtrip!(
    midi_event_note_on,
    MidiEvent::NoteOn { position: 0, note: 60, velocity: 100, channel: 0 },
    MidiEvent
);

serde_roundtrip!(
    midi_clip,
    MidiClip::new("test", 0, 44100),
    MidiClip
);

serde_roundtrip!(
    note_on_v2,
    NoteOnV2 { position: 0, note: 60, velocity: 32768, channel: 0, attribute_type: 0, attribute_data: 0 },
    NoteOnV2
);

serde_roundtrip!(
    cc_v2,
    ControlChangeV2 { position: 0, controller: 1, value: u32::MAX, channel: 0 },
    ControlChangeV2
);

serde_roundtrip!(ump_type, UmpMessageType::Midi2ChannelVoice, UmpMessageType);

serde_roundtrip!(
    velocity_curve_soft,
    VelocityCurve::Soft,
    VelocityCurve
);

serde_roundtrip!(
    velocity_curve_fixed,
    VelocityCurve::Fixed(80),
    VelocityCurve
);

serde_roundtrip!(
    cc_mapping,
    CcMapping::new(1, 0, 0.0, 1.0),
    CcMapping
);

serde_roundtrip!(
    capture_config,
    CaptureConfig::default(),
    CaptureConfig
);

serde_roundtrip!(
    output_config,
    OutputConfig::default(),
    OutputConfig
);

serde_roundtrip!(
    audio_device,
    AudioDevice { id: 1, name: "Test".into(), device_type: DeviceType::Source, channels: 2, sample_rate: 48000 },
    AudioDevice
);

serde_roundtrip!(
    recording_mode,
    RecordingMode::Overdub,
    RecordingMode
);

serde_roundtrip!(
    biquad_coeffs,
    BiquadCoeffs::design(FilterType::LowPass, 1000.0, 0.707, 44100),
    BiquadCoeffs
);
