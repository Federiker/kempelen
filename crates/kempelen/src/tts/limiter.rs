//! A transparent peak limiter for the model's output.
//!
//! Kokoro regularly produces peaks above full scale (up to about 1.4 with some voices). Writing
//! them as 16-bit PCM clips them flat, which is heard as crackle, and the later MP3 encode
//! overshoots again. Instead of clipping, the gain dips smoothly around each over-range sample,
//! leaving everything else untouched.

/// Highest sample magnitude after limiting, about -1 dBFS: headroom for MP3 and resampling overshoot.
pub const CEILING: f32 = 0.89;

/// Half-width of the gain dip in samples (1.5 ms at 24 kHz).
const RADIUS: usize = 36;

/// A sample's result depends on the input this far either side: the dip around a peak reaches
/// `RADIUS`, and the smoothing averages another `RADIUS` of dips.
const CONTEXT: usize = 2 * RADIUS;

/// Samples gathered before a batch is limited, beyond the context still to come.
const BATCH: usize = 8_192;

/// Reduce samples above [`CEILING`] with a short smooth gain dip instead of clipping them.
///
/// Samples below the ceiling away from any peak are left bit-identical.
pub fn limit_peaks(samples: &mut [f32]) {
    limit_peaks_to(samples, CEILING);
}

/// Like [`limit_peaks`], with an explicit `ceiling` (linear, at most 1.0).
#[expect(clippy::similar_names, reason = "the limiter and what it limited")]
pub fn limit_peaks_to(samples: &mut [f32], ceiling: f32) {
    let mut limiter = PeakLimiter::new(ceiling);
    let mut limited = Vec::with_capacity(samples.len());
    limiter.push(samples, &mut limited);
    limiter.finish(&mut limited);
    samples.copy_from_slice(&limited);
}

/// [`limit_peaks_to`] for audio that arrives a piece at a time: the same result, sample for
/// sample, whatever the pieces, holding only a few thousand samples.
///
/// Each over-range sample needs a gain of at most `ceiling / |sample|` across the dip radius
/// around it; every sample within the radius of such a dip is multiplied by the box average of
/// those gains over the same radius. That reaches the required depth at every over-range sample,
/// because all gains within its radius are at least that deep.
pub struct PeakLimiter {
    ceiling: f32,
    /// Unlimited input from global index `start`: the context before `next` and what follows.
    input: Vec<f32>,
    start: usize,
    /// Global index of the first sample not yet handed out.
    next: usize,
}

impl PeakLimiter {
    pub fn new(ceiling: f32) -> Self {
        Self {
            ceiling,
            input: Vec::new(),
            start: 0,
            next: 0,
        }
    }

    /// Add `samples`; the ones that can no longer change are appended to `limited`.
    pub fn push(&mut self, samples: &[f32], limited: &mut Vec<f32>) {
        self.input.extend_from_slice(samples);
        let known = self.start + self.input.len();
        if known >= self.next + CONTEXT + BATCH {
            self.release(known - CONTEXT, None, limited);
        }
    }

    /// Hand out the rest: the input has ended.
    pub fn finish(mut self, limited: &mut Vec<f32>) {
        let length = self.start + self.input.len();
        self.release(length, Some(length), limited);
    }

    /// Limit and hand out the samples up to global index `end`; `length` is the whole input's
    /// length once it is known.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a dip spans a few hundred samples"
    )]
    fn release(&mut self, end: usize, length: Option<usize>, limited: &mut Vec<f32>) {
        let (from, to) = (self.next, end);
        let sample = |at: usize| self.input[at - self.start];
        // Gains are needed within `RADIUS` of the samples handed out, and depend on peaks within
        // `RADIUS` of those.
        let gains_from = from.saturating_sub(RADIUS);
        let gains_to = (to + RADIUS).min(length.unwrap_or(usize::MAX));
        let peaks_from = from.saturating_sub(CONTEXT).max(self.start);
        let peaks_to = (to + CONTEXT).min(self.start + self.input.len());
        let mut gains = vec![1.0f32; gains_to - gains_from];
        let mut dipped = vec![false; gains_to - gains_from];
        let mut any = false;
        for index in peaks_from..peaks_to {
            let magnitude = sample(index).abs();
            if magnitude <= self.ceiling {
                continue;
            }
            any = true;
            let gain = self.ceiling / magnitude;
            let first = index.saturating_sub(RADIUS).max(gains_from);
            let last = (index + RADIUS + 1).min(gains_to);
            for at in first..last {
                gains[at - gains_from] = gains[at - gains_from].min(gain);
                dipped[at - gains_from] = true;
            }
        }
        limited.reserve(to - from);
        for at in from..to {
            let value = sample(at);
            if !any || !dipped[at - gains_from] {
                limited.push(value);
                continue;
            }
            let first = at.saturating_sub(RADIUS);
            let last = match length {
                Some(length) => (at + RADIUS).min(length - 1),
                None => at + RADIUS,
            };
            let average = (first..=last)
                .map(|near| gains[near - gains_from])
                .sum::<f32>()
                / (last - first + 1) as f32;
            // The explicit clamp only absorbs floating-point rounding at the ceiling.
            limited.push((value * average).clamp(-self.ceiling, self.ceiling));
        }
        self.next = to;
        let keep_from = to.saturating_sub(CONTEXT).max(self.start);
        self.input.drain(..keep_from - self.start);
        self.start = keep_from;
    }
}

#[cfg(test)]
#[path = "../../tests/unit/tts/limiter.rs"]
mod tests;
