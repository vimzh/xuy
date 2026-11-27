use dioxus::prelude::*;

const CLOCK_SCRIPT: &str = r#"
    const clock = document.getElementById('starter-clock');
    const [time, date] = clock.children;

    function updateClock() {
        if (!clock.isConnected) return;

        const now = new Date();
        time.dateTime = now.toISOString();
        time.textContent = new Intl.DateTimeFormat(undefined, {
            hour: 'numeric',
            minute: '2-digit',
        }).formatToParts(now)
            .filter(part => part.type !== 'dayPeriod')
            .map(part => part.value)
            .join('')
            .trim();
        const month = String(now.getMonth() + 1).padStart(2, '0');
        const day = String(now.getDate()).padStart(2, '0');
        date.dateTime = `${now.getFullYear()}-${month}-${day}`;
        date.textContent = now.toLocaleDateString(undefined, {
            weekday: 'short',
            month: 'short',
            day: 'numeric',
            year: 'numeric',
        });
        setTimeout(updateClock, 60_000 - Date.now() % 60_000);
    }

    updateClock();
"#;

#[component]
pub(crate) fn StarterClock() -> Element {
    rsx! {
        div {
            id: "starter-clock",
            onmounted: async move |_| {
                if let Err(error) = document::eval(CLOCK_SCRIPT).await {
                    eprintln!("Unable to start the local clock: {error}");
                }
            },
            time {}
            time { class: "starter-date" }
        }
    }
}
