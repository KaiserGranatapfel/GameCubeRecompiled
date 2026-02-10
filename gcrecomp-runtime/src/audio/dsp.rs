/// DSP processor — voice management and Nintendo ADPCM decoding.
use log::info;

/// State for a single DSP voice.
#[derive(Debug, Clone)]
pub struct DspVoice {
    pub active: bool,
    pub data_addr: u32,
    pub data_len: u32,
    pub loop_addr: u32,
    pub loop_flag: bool,
    pub sample_rate: u32,
    pub volume_left: i16,
    pub volume_right: i16,
    /// ADPCM decoder state
    pub adpcm_state: AdpcmState,
    /// 16 ADPCM coefficients
    pub coefficients: [i16; 16],
}

#[derive(Debug, Clone, Default)]
pub struct AdpcmState {
    pub hist1: i16,
    pub hist2: i16,
}

impl Default for DspVoice {
    fn default() -> Self {
        Self {
            active: false,
            data_addr: 0,
            data_len: 0,
            loop_addr: 0,
            loop_flag: false,
            sample_rate: 32000,
            volume_left: 0x7FFF,
            volume_right: 0x7FFF,
            adpcm_state: AdpcmState::default(),
            coefficients: [0; 16],
        }
    }
}

pub struct DspProcessor {
    pub voices: Vec<DspVoice>,
    pub initialized: bool,
}

impl DspProcessor {
    pub fn new() -> Self {
        Self {
            voices: (0..64).map(|_| DspVoice::default()).collect(),
            initialized: false,
        }
    }

    /// DSPInit
    pub fn init(&mut self) {
        info!("DSPInit");
        self.initialized = true;
    }

    /// Decode a block of Nintendo DSP-ADPCM data into PCM samples.
    ///
    /// Each DSP-ADPCM frame is 8 bytes and decodes to 14 samples.
    /// Byte 0: header (high nibble = predictor index, low nibble = scale)
    /// Bytes 1-7: 14 nibbles of compressed sample data.
    pub fn decode_adpcm(data: &[u8], coefficients: &[i16; 16], state: &mut AdpcmState) -> Vec<i16> {
        let mut output = Vec::new();

        for frame in data.chunks(8) {
            if frame.len() < 8 {
                break;
            }

            let header = frame[0];
            let predictor_index = ((header >> 4) & 0x7) as usize;
            let scale = 1i32 << (header & 0xF);

            let coef_idx = predictor_index * 2;
            let coef1 = if coef_idx < 16 {
                coefficients[coef_idx] as i32
            } else {
                0
            };
            let coef2 = if coef_idx + 1 < 16 {
                coefficients[coef_idx + 1] as i32
            } else {
                0
            };

            for byte in &frame[1..8] {
                let byte = *byte;
                for nibble in 0..2 {
                    let raw = if nibble == 0 {
                        ((byte >> 4) & 0xF) as i8
                    } else {
                        (byte & 0xF) as i8
                    };

                    // Sign-extend 4-bit nibble
                    let signed = if raw >= 8 {
                        raw as i32 - 16
                    } else {
                        raw as i32
                    };

                    let scaled = signed * scale;
                    let predicted = scaled
                        + ((coef1 * state.hist1 as i32) >> 11)
                        + ((coef2 * state.hist2 as i32) >> 11);

                    // Clamp to i16 range
                    let sample = predicted.clamp(-32768, 32767) as i16;

                    state.hist2 = state.hist1;
                    state.hist1 = sample;

                    output.push(sample);
                }
            }
        }

        output
    }
}

impl Default for DspProcessor {
    fn default() -> Self {
        Self::new()
    }
}
