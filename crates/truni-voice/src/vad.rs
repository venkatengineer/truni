pub struct VadDetector {
    energy_threshold: f32,
    silence_frames_needed: usize,
    speech_frames_seen: usize,
    silence_frames_seen: usize,
    in_speech: bool,
}

impl Default for VadDetector {
    fn default() -> Self {
        Self::new(0.015, 30) // ~1.0 second silence at 30ms frames
    }
}

impl VadDetector {
    pub fn new(energy_threshold: f32, silence_frames_needed: usize) -> Self {
        Self {
            energy_threshold,
            silence_frames_needed,
            speech_frames_seen: 0,
            silence_frames_seen: 0,
            in_speech: false,
        }
    }

    pub fn reset(&mut self) {
        self.speech_frames_seen = 0;
        self.silence_frames_seen = 0;
        self.in_speech = false;
    }

    /// Process a frame of 16-bit signed PCM samples.
    /// Returns: (is_voice_active, user_finished_speaking)
    pub fn process_pcm16(&mut self, samples: &[i16]) -> (bool, bool) {
        if samples.is_empty() {
            return (false, false);
        }

        // Calculate Root Mean Square (RMS) energy normalized to [0.0, 1.0]
        let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        let rms = (sum_sq / (samples.len() as f64)).sqrt() / 32768.0;
        let is_speech = (rms as f32) > self.energy_threshold;

        let mut finished = false;

        if is_speech {
            self.speech_frames_seen += 1;
            self.silence_frames_seen = 0;
            if self.speech_frames_seen >= 3 {
                self.in_speech = true;
            }
        } else if self.in_speech {
            self.silence_frames_seen += 1;
            if self.silence_frames_seen >= self.silence_frames_needed {
                self.in_speech = false;
                finished = true;
            }
        }

        (self.in_speech, finished)
    }

    pub fn is_loud_interruption(&self, samples: &[i16]) -> bool {
        if samples.is_empty() {
            return false;
        }
        let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        let rms = (sum_sq / (samples.len() as f64)).sqrt() / 32768.0;
        // Higher threshold for interruption to avoid false triggers from speaker echo
        (rms as f32) > (self.energy_threshold * 2.5)
    }
}
