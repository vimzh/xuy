//! On-demand CPU and memory readings for the xuy process.

use std::{
    io,
    time::{Duration, Instant},
};

use dioxus::prelude::{dioxus_core::use_drop, *};

const SAMPLE_CLOCK: &str = r#"
    let timer;
    function updateSampling() {
        clearInterval(timer);
        dioxus.send(!document.hidden);
        if (!document.hidden) timer = setInterval(() => dioxus.send(true), 1000);
    }

    try {
        document.addEventListener("visibilitychange", updateSampling);
        updateSampling();
        await dioxus.recv();
    } finally {
        clearInterval(timer);
        document.removeEventListener("visibilitychange", updateSampling);
    }
"#;

struct Snapshot {
    at: Instant,
    cpu_time: Duration,
    memory_bytes: u64,
}

impl Snapshot {
    #[cfg(target_os = "macos")]
    fn read() -> io::Result<Self> {
        let mut cpu = std::mem::MaybeUninit::<libc::timespec>::uninit();
        let mut usage = std::mem::MaybeUninit::<libc::rusage_info_v0>::uninit();
        // Both APIs initialize caller-owned buffers on success. The usage buffer matches V0.
        let (cpu, usage) = unsafe {
            if libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, cpu.as_mut_ptr()) != 0
                || libc::proc_pid_rusage(
                    libc::getpid(),
                    libc::RUSAGE_INFO_V0,
                    usage.as_mut_ptr().cast(),
                ) != 0
            {
                return Err(io::Error::last_os_error());
            }
            (cpu.assume_init(), usage.assume_init())
        };
        Ok(Self {
            at: Instant::now(),
            cpu_time: Duration::new(cpu.tv_sec as u64, cpu.tv_nsec as u32),
            memory_bytes: usage.ri_phys_footprint,
        })
    }

    #[cfg(not(target_os = "macos"))]
    fn read() -> io::Result<Self> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    fn cpu_percent_since(&self, previous: &Self) -> Option<f64> {
        let elapsed = self.at.checked_duration_since(previous.at)?;
        let cpu = self.cpu_time.checked_sub(previous.cpu_time)?;
        (!elapsed.is_zero()).then(|| 100.0 * cpu.as_secs_f64() / elapsed.as_secs_f64())
    }
}

enum Reading {
    Waiting,
    Available {
        cpu_percent: Option<f64>,
        memory_bytes: u64,
    },
    Unavailable,
}

#[component]
pub(crate) fn PerformancePanel() -> Element {
    let mut reading = use_signal(|| Reading::Waiting);
    let mut bridge = use_signal(|| None::<document::Eval>);
    use_drop(move || {
        if let Some(clock) = *bridge.peek() {
            let _ = clock.send(());
        }
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/performance.css") }
        aside { class: "performance-panel", aria_label: "xuy process performance",
            onmounted: move |_| async move {
                let mut clock = document::eval(SAMPLE_CLOCK);
                bridge.set(Some(clock));
                let mut previous = None;
                while let Ok(visible) = clock.recv::<bool>().await {
                    if !visible {
                        previous = None;
                        reading.set(Reading::Waiting);
                        continue;
                    }
                    match Snapshot::read() {
                        Ok(snapshot) => {
                            reading.set(Reading::Available {
                                cpu_percent: previous.as_ref().and_then(|last| snapshot.cpu_percent_since(last)),
                                memory_bytes: snapshot.memory_bytes,
                            });
                            previous = Some(snapshot);
                        }
                        Err(error) => {
                            eprintln!("Could not read xuy process metrics: {error}");
                            reading.set(Reading::Unavailable);
                            let _ = clock.send(());
                            break;
                        }
                    }
                }
            },
            match &*reading.read() {
                Reading::Unavailable => rsx! { span { "Performance metrics unavailable" } },
                state => {
                    let (cpu, memory) = match state {
                        Reading::Available { cpu_percent, memory_bytes } => (
                            cpu_percent.map_or_else(|| "—".to_owned(), |percent| format!("{percent:.1}%")),
                            format!("{:.1} MiB", *memory_bytes as f64 / (1024.0 * 1024.0)),
                        ),
                        _ => ("—".to_owned(), "—".to_owned()),
                    };
                    rsx! {
                        span { title: "xuy CPU usage. 100% is one fully used core.", "CPU " strong { "{cpu}" } }
                        span { title: "xuy process physical memory footprint", "Mem " strong { "{memory}" } }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_percentage_uses_elapsed_time_and_allows_multicore_usage() {
        let baseline = Snapshot {
            at: Instant::now(),
            cpu_time: Duration::from_secs(2),
            memory_bytes: 0,
        };
        let next = Snapshot {
            at: baseline.at + Duration::from_secs(1),
            cpu_time: Duration::from_millis(3500),
            memory_bytes: 0,
        };
        assert_eq!(next.cpu_percent_since(&baseline), Some(150.0));
        assert_eq!(baseline.cpu_percent_since(&baseline), None);
        assert_eq!(baseline.cpu_percent_since(&next), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reads_the_current_process_footprint() {
        assert!(
            Snapshot::read()
                .expect("current process metrics")
                .memory_bytes
                > 0
        );
    }
}
