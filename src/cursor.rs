//! Editor cursor preferences, including compatibility with saved shape and blink settings.

use dioxus::prelude::{dioxus_core::spawn_forever, *};

use crate::command_bar::Command;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum CursorShape {
    #[default]
    Bar,
    Block,
    Underline,
}

impl CursorShape {
    const OPTIONS: [(Self, &'static str, &'static str); 3] = [
        (Self::Bar, "bar", "Bar"),
        (Self::Block, "block", "Block"),
        (Self::Underline, "underline", "Underline"),
    ];

    pub(crate) fn id(self) -> &'static str {
        Self::OPTIONS[self as usize].1
    }

    fn parse(id: &str) -> Option<Self> {
        Self::OPTIONS
            .iter()
            .find_map(|(shape, value, _)| (*value == id).then_some(*shape))
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum BlinkStyle {
    #[default]
    Sharp,
    Fade,
}

impl BlinkStyle {
    const OPTIONS: [(Self, &'static str, &'static str); 2] = [
        (Self::Sharp, "sharp", "Sharp"),
        (Self::Fade, "fade", "Fade"),
    ];

    pub(crate) fn id(self) -> &'static str {
        Self::OPTIONS[self as usize].1
    }

    fn parse(id: &str) -> Option<Self> {
        Self::OPTIONS
            .iter()
            .find_map(|(style, value, _)| (*value == id).then_some(*style))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CursorColor(u32);

impl CursorColor {
    fn parse(value: &str) -> Option<Self> {
        let hex = value.strip_prefix('#')?;
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(hex, 16).ok().map(Self)
    }

    fn hex(self) -> String {
        format!("#{:06x}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct CursorOptions {
    pub(crate) shape: CursorShape,
    pub(crate) blink: bool,
    pub(crate) smooth: bool,
    pub(crate) blink_style: BlinkStyle,
    color: Option<CursorColor>,
}

#[derive(Clone, Copy)]
enum CursorSetting {
    Shape(CursorShape),
    Smooth(bool),
    Blink(bool),
    Style(BlinkStyle),
    CustomColor(bool),
}

impl CursorSetting {
    fn apply(self, mut options: CursorOptions) -> CursorOptions {
        match self {
            Self::Shape(shape) => options.shape = shape,
            Self::Smooth(smooth) => options.smooth = smooth,
            Self::Blink(blink) => options.blink = blink,
            Self::Style(style) => options.blink_style = style,
            Self::CustomColor(true) => {
                options.color.get_or_insert(CursorColor(0x808080));
            }
            Self::CustomColor(false) => options.color = None,
        }
        options
    }

    fn available(self, options: CursorOptions) -> bool {
        options.blink || !matches!(self, Self::Style(_))
    }

    fn keywords(self) -> &'static str {
        match self {
            Self::Shape(_) => "settings editor cursor caret shape",
            Self::Smooth(_) => "settings editor cursor caret movement motion",
            Self::Blink(_) | Self::Style(_) => "settings editor cursor caret blink animation",
            Self::CustomColor(_) => "settings editor cursor caret color appearance",
        }
    }
}

const CURSOR_COMMANDS: [(&str, &str, CursorSetting); 11] = {
    use CursorSetting::*;
    [
        ("cursor-shape-bar", "Bar Cursor", Shape(CursorShape::Bar)),
        (
            "cursor-shape-block",
            "Block Cursor",
            Shape(CursorShape::Block),
        ),
        (
            "cursor-shape-underline",
            "Underline Cursor",
            Shape(CursorShape::Underline),
        ),
        (
            "cursor-immediate",
            "Immediate Cursor Movement",
            Smooth(false),
        ),
        ("cursor-smooth", "Smooth Cursor Movement", Smooth(true)),
        ("cursor-blink-on", "Enable Cursor Blink", Blink(true)),
        ("cursor-blink-off", "Disable Cursor Blink", Blink(false)),
        (
            "cursor-blink-sharp",
            "Sharp Cursor Blink",
            Style(BlinkStyle::Sharp),
        ),
        (
            "cursor-blink-fade",
            "Fade Cursor Blink",
            Style(BlinkStyle::Fade),
        ),
        (
            "cursor-color-custom",
            "Use Custom Cursor Color",
            CustomColor(true),
        ),
        (
            "cursor-color-theme",
            "Use Theme Cursor Color",
            CustomColor(false),
        ),
    ]
};

pub(crate) fn cursor_commands(options: CursorOptions) -> impl Iterator<Item = Command> {
    CURSOR_COMMANDS
        .into_iter()
        .filter(move |(_, _, setting)| setting.available(options))
        .map(move |(id, label, setting)| {
            Command::new(id, label, "settings-01", "", setting.keywords())
                .setting(setting.apply(options) == options)
        })
}

pub(crate) fn apply_cursor_command(id: &str, cursor: Signal<Option<CursorOptions>>) -> bool {
    let options = cursor.peek().unwrap_or_default();
    if let Some((_, _, setting)) = CURSOR_COMMANDS
        .iter()
        .find(|(command_id, _, setting)| *command_id == id && setting.available(options))
    {
        change_cursor(cursor, *setting);
        true
    } else {
        false
    }
}

impl Default for CursorOptions {
    fn default() -> Self {
        Self {
            shape: CursorShape::Bar,
            blink: true,
            smooth: false,
            blink_style: BlinkStyle::Sharp,
            color: None,
        }
    }
}

type StoredCursor = (String, bool, Option<(bool, Option<String>, String)>);

impl CursorOptions {
    fn restore((id, blink, details): StoredCursor) -> Option<Self> {
        let shape = CursorShape::parse(&id)?;
        let mut options = Self {
            shape,
            blink,
            ..Self::default()
        };
        if let Some((smooth, color, style)) = details {
            options.smooth = smooth;
            options.color = match color {
                Some(value) => Some(CursorColor::parse(&value)?),
                None => None,
            };
            options.blink_style = BlinkStyle::parse(&style)?;
        }
        Some(options)
    }

    pub(crate) fn color(self) -> String {
        self.color
            .map_or_else(|| "currentColor".into(), CursorColor::hex)
    }
}

pub(crate) fn use_cursor_options() -> Signal<Option<CursorOptions>> {
    let mut cursor = use_signal(|| None);
    use_future(move || async move {
        // Wait for Rust's acknowledgment so Desktop keeps the query alive until we read it.
        let mut load = document::eval(
            "try {
                const value = JSON.parse(localStorage.getItem('xuy.cursor'));
                dioxus.send([value === null ? null : [value[0], value[1], value[2] ?? null], null]);
            } catch (error) {
                dioxus.send([null, String(error)]);
            }
            await dioxus.recv();",
        );
        let result = load.recv::<(Option<StoredCursor>, Option<String>)>().await;
        let _ = load.send(());
        match result {
            Ok((Some(saved), None)) => match CursorOptions::restore(saved) {
                Some(options) if cursor.peek().is_none() => cursor.set(Some(options)),
                Some(_) => {}
                None => eprintln!("Could not restore cursor settings: invalid preference"),
            },
            Ok((_, Some(error))) => eprintln!("Could not restore cursor settings: {error}"),
            Ok((None, None)) => {}
            Err(error) => eprintln!("Could not restore cursor settings: {error}"),
        }
    });
    cursor
}

fn save_cursor(mut cursor: Signal<Option<CursorOptions>>, options: CursorOptions) {
    cursor.set(Some(options));
    let color = options
        .color
        .map_or_else(|| "null".into(), |color| format!("'{}'", color.hex()));
    let mut save = document::eval(&format!(
        "try {{
            localStorage.setItem('xuy.cursor', JSON.stringify(['{}', {}, [{}, {}, '{}']]));
            dioxus.send(null);
        }} catch (error) {{
            dioxus.send(String(error));
        }}
        await dioxus.recv();",
        options.shape.id(),
        options.blink,
        options.smooth,
        color,
        options.blink_style.id(),
    ));
    spawn_forever(async move {
        let result = save.recv::<Option<String>>().await;
        let _ = save.send(());
        match result {
            Ok(None) => {}
            Ok(Some(error)) => eprintln!("Could not save cursor settings: {error}"),
            Err(error) => eprintln!("Could not save cursor settings: {error}"),
        }
    });
}

fn change_cursor(cursor: Signal<Option<CursorOptions>>, setting: CursorSetting) {
    let options = cursor.peek().unwrap_or_default();
    save_cursor(cursor, setting.apply(options));
}

#[component]
pub(crate) fn CursorSettings(cursor: Signal<Option<CursorOptions>>) -> Element {
    let options = cursor().unwrap_or_default();
    rsx! {
        label {
            span { "Cursor shape" }
            select {
                onchange: move |event| {
                    if let Some(shape) = CursorShape::parse(&event.value()) {
                        change_cursor(cursor, CursorSetting::Shape(shape));
                    }
                },
                for (shape, id, label) in CursorShape::OPTIONS {
                    option { value: id, selected: shape == options.shape, "{label}" }
                }
            }
        }
        label {
            span { "Cursor movement" }
            select {
                onchange: move |event| {
                    change_cursor(cursor, CursorSetting::Smooth(event.value() == "smooth"));
                },
                option { value: "immediate", selected: !options.smooth, "Immediate" }
                option { value: "smooth", selected: options.smooth, "Smooth" }
            }
        }
        label {
            span { "Custom cursor color" }
            input {
                r#type: "checkbox",
                checked: options.color.is_some(),
                onchange: move |event| {
                    change_cursor(cursor, CursorSetting::CustomColor(event.checked()));
                },
            }
        }
        if let Some(color) = options.color {
            label {
                span { "Cursor color" }
                input {
                    r#type: "color",
                    value: color.hex(),
                    onchange: move |event| {
                        if let Some(color) = CursorColor::parse(&event.value()) {
                            let current = cursor.peek().unwrap_or_default();
                            save_cursor(cursor, CursorOptions { color: Some(color), ..current });
                        }
                    },
                }
            }
        }
        label {
            span { "Blink cursor" }
            input {
                r#type: "checkbox",
                checked: options.blink,
                onchange: move |event| {
                    change_cursor(cursor, CursorSetting::Blink(event.checked()));
                },
            }
        }
        label {
            span { "Blink style" }
            select {
                disabled: !options.blink,
                onchange: move |event| {
                    if let Some(blink_style) = BlinkStyle::parse(&event.value()) {
                        change_cursor(cursor, CursorSetting::Style(blink_style));
                    }
                },
                for (style, id, label) in BlinkStyle::OPTIONS {
                    option { value: id, selected: style == options.blink_style, "{label}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlinkStyle, CursorColor, CursorOptions, CursorSetting, CursorShape, cursor_commands,
    };

    #[test]
    fn restores_both_old_and_current_preferences() {
        let old = CursorOptions::restore(("block".into(), false, None)).unwrap();
        assert_eq!(old.shape.id(), "block");
        assert!(!old.blink && !old.smooth);
        assert_eq!(old.blink_style.id(), "sharp");
        assert_eq!(old.color(), "currentColor");

        let current = CursorOptions::restore((
            "underline".into(),
            true,
            Some((true, Some("#45AB03".into()), "fade".into())),
        ))
        .unwrap();
        assert!(current.shape == CursorShape::Underline && current.blink_style == BlinkStyle::Fade);
        assert!(current.blink && current.smooth);
        assert_eq!(current.color(), "#45ab03");
    }

    #[test]
    fn rejects_non_hex_color_values() {
        for color in ["#12abcd;", "#+1abcd", "#123", "red"] {
            assert!(
                CursorOptions::restore((
                    "bar".into(),
                    true,
                    Some((false, Some(color.into()), "sharp".into())),
                ))
                .is_none()
            );
        }
    }

    #[test]
    fn palette_respects_blink_state_and_preserves_a_selected_color() {
        let options = CursorOptions {
            color: Some(CursorColor(0x45ab03)),
            ..CursorOptions::default()
        };
        assert!(cursor_commands(options).any(|command| command.id == "cursor-blink-fade"));
        let still = CursorSetting::Blink(false).apply(options);
        assert!(!cursor_commands(still).any(|command| command.id == "cursor-blink-fade"));
        assert_eq!(
            CursorSetting::CustomColor(true).apply(still).color(),
            "#45ab03"
        );
    }
}
