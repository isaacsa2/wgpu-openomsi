use std::{
    sync::{Mutex, OnceLock},
    time::Instant,
};

pub(super) struct Sample {
    last: Instant,
    phases: [f64; 5],
}

#[derive(Default)]
struct Totals {
    since: Option<Instant>,
    submits: u64,
    buffers: u64,
    commands: u64,
    phases: [f64; 5],
    worst: f64,
}

impl Sample {
    pub(super) fn begin() -> Option<Self> {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        ENABLED
            .get_or_init(|| std::env::var("WGPU_GL_PROFILE").is_ok_and(|v| v == "1"))
            .then(|| Self {
                last: Instant::now(),
                phases: [0.0; 5],
            })
    }

    pub(super) fn mark(&mut self, phase: usize) {
        let now = Instant::now();
        self.phases[phase] += now.duration_since(self.last).as_secs_f64();
        self.last = now;
    }

    pub(super) fn finish(self, buffers: usize, commands: usize) {
        static TOTALS: Mutex<Totals> = Mutex::new(Totals {
            since: None,
            submits: 0,
            buffers: 0,
            commands: 0,
            phases: [0.0; 5],
            worst: 0.0,
        });
        let mut t = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
        let since = *t.since.get_or_insert_with(Instant::now);
        t.submits += 1;
        t.buffers += buffers as u64;
        t.commands += commands as u64;
        t.worst = t.worst.max(self.phases.iter().sum());
        for (total, phase) in t.phases.iter_mut().zip(self.phases) {
            *total += phase;
        }
        if since.elapsed().as_secs_f64() < 1.0 {
            return;
        }
        let n = t.submits as f64;
        let ms = t.phases.map(|s| s * 1000.0 / n);
        log::info!(
            "GL submit: {} calls, {:.1} buffers/call, {:.0} commands/call; ms/call context={:.3} reset={:.3} commands={:.3} fence={:.3} flush={:.3}; worst={:.3} ms",
            t.submits, t.buffers as f64 / n, t.commands as f64 / n,
            ms[0], ms[1], ms[2], ms[3], ms[4], t.worst * 1000.0,
        );
        *t = Totals::default();
    }
}
