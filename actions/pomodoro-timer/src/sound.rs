// specs/006_pomodoro_timer.md
use rodio::source::SineWave;
use rodio::{OutputStream, Sink, Source};
use std::time::Duration;

pub fn play_focus_complete(volume_pct: u8) {
    play_tones(vec![(587.33, 150), (880.0, 300)], volume_pct);
}

pub fn play_break_complete(volume_pct: u8) {
    play_tones(vec![(440.0, 120), (554.37, 120), (659.25, 250)], volume_pct);
}

fn play_tones(notes: Vec<(f32, u64)>, volume_pct: u8) {
    if volume_pct == 0 {
        return;
    }
    let gain = (volume_pct.min(100) as f32) / 100.0 * 0.4; // Safety gain cap

    std::thread::spawn(move || {
        let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
            return;
        };
        let Ok(sink) = Sink::try_new(&stream_handle) else {
            return;
        };

        for (freq, duration_ms) in notes {
            let source = SineWave::new(freq)
                .take_duration(Duration::from_millis(duration_ms))
                .amplify(gain);
            sink.append(source);
        }

        sink.sleep_until_end();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_chime_functions_do_not_panic() {
        // Zero volume or normal volume should safely return without panic
        play_focus_complete(0);
        play_break_complete(0);
    }
}
