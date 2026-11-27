//! Saved font, line spacing, and code theme shared by Settings and the command bar.

use dioxus::prelude::*;

use crate::commands::palette::Command;

#[cfg(target_os = "macos")]
use objc2_foundation::{NSString, NSUserDefaults};

#[derive(Clone, Copy, PartialEq, Eq)]
enum WritingPreference {
    Font,
    LineSpacing,
    CodeTheme,
}

// Each choice has a stable command ID, a display label, and a CSS value.
type Choice = (&'static str, &'static str, &'static str);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct WritingStyle([Choice; 3]);

impl WritingStyle {
    pub(crate) fn load() -> Self {
        Self(WritingPreference::ALL.map(WritingPreference::load))
    }

    pub(crate) fn font(self) -> &'static str {
        self.0[WritingPreference::Font as usize].2
    }

    pub(crate) fn code_font(self) -> &'static str {
        let (id, _, value) = self.0[WritingPreference::Font as usize];
        if id == "font-helvetica" {
            WritingPreference::Font.choices()[0].2
        } else {
            value
        }
    }

    pub(crate) fn line_height(self) -> &'static str {
        self.0[WritingPreference::LineSpacing as usize].2
    }

    pub(crate) fn code_theme(self) -> &'static str {
        self.0[WritingPreference::CodeTheme as usize].2
    }
}

impl WritingPreference {
    const ALL: [Self; 3] = [Self::Font, Self::LineSpacing, Self::CodeTheme];

    fn label(self) -> &'static str {
        match self {
            Self::Font => "Writing Font",
            Self::LineSpacing => "Line Spacing",
            Self::CodeTheme => "Code Block Theme",
        }
    }

    #[cfg(target_os = "macos")]
    fn key(self) -> &'static str {
        match self {
            Self::Font => "xuy.writing-font",
            Self::LineSpacing => "xuy.line-spacing",
            Self::CodeTheme => "xuy.code-block-theme",
        }
    }

    fn choices(self) -> &'static [Choice] {
        match self {
            Self::Font => &[
                (
                    "font-ia-mono",
                    "iA Writer Mono",
                    "'iA Writer Mono', Menlo, monospace",
                ),
                (
                    "font-ia-duo",
                    "iA Writer Duo",
                    "'iA Writer Duo', Menlo, monospace",
                ),
                (
                    "font-ia-quattro",
                    "iA Writer Quattro",
                    "'iA Writer Quattro', Menlo, monospace",
                ),
                (
                    "font-helvetica",
                    "Helvetica Neue",
                    "'Helvetica Neue', sans-serif",
                ),
                (
                    "font-jetbrains",
                    "JetBrains Mono",
                    "'JetBrains Mono', Menlo, monospace",
                ),
            ],
            Self::LineSpacing => &[
                ("spacing-standard", "Standard", "1.7"),
                ("spacing-compact", "Compact", "1.4"),
                ("spacing-relaxed", "Relaxed", "2"),
            ],
            Self::CodeTheme => &[
                ("code-theme-system", "System", "system"),
                ("code-theme-paper", "Paper", "paper"),
                ("code-theme-graphite", "Graphite", "graphite"),
            ],
        }
    }

    fn find(self, id: &str) -> Option<Choice> {
        self.choices().iter().copied().find(|choice| choice.0 == id)
    }

    fn load(self) -> Choice {
        #[cfg(target_os = "macos")]
        if let Some(saved) =
            NSUserDefaults::standardUserDefaults().stringForKey(&NSString::from_str(self.key()))
            && let Some(choice) = self.find(&saved.to_string())
        {
            return choice;
        }
        self.choices()[0]
    }

    fn set(self, mut style: Signal<WritingStyle>, choice: Choice) {
        style.write().0[self as usize] = choice;
        #[cfg(target_os = "macos")]
        unsafe {
            NSUserDefaults::standardUserDefaults().setObject_forKey(
                Some(&NSString::from_str(choice.0)),
                &NSString::from_str(self.key()),
            );
        }
    }
}

pub(crate) fn style_commands(style: WritingStyle) -> impl Iterator<Item = Command> {
    WritingPreference::ALL
        .into_iter()
        .flat_map(move |preference| {
            preference.choices().iter().map(move |&(id, label, _)| {
                let keywords = match preference {
                    WritingPreference::Font => "settings editor markdown typeface",
                    WritingPreference::LineSpacing => "settings editor markdown line height",
                    WritingPreference::CodeTheme => "settings markdown syntax highlighting color",
                };
                Command::new(id, preference.label(), "settings-01", "", keywords)
                    .setting_value(label, style.0[preference as usize].0 == id)
            })
        })
}

pub(crate) fn apply_style_command(id: &str, style: Signal<WritingStyle>) -> bool {
    WritingPreference::ALL.into_iter().any(|preference| {
        if let Some(choice) = preference.find(id) {
            preference.set(style, choice);
            true
        } else {
            false
        }
    })
}

#[component]
pub(crate) fn WritingStyleSettings(style: Signal<WritingStyle>) -> Element {
    let current = style();
    rsx! {
        for preference in WritingPreference::ALL {
            label { key: "{preference as u8}",
                span { "{preference.label()}" }
                select {
                    onchange: move |event| {
                        if let Some(choice) = preference.find(&event.value()) {
                            preference.set(style, choice);
                        }
                    },
                    for &(id, label, _) in preference.choices() {
                        option { value: id, selected: current.0[preference as usize].0 == id, "{label}" }
                    }
                }
            }
        }
    }
}
