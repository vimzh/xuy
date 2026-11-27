use std::{hash::BuildHasher, hash::RandomState, sync::OnceLock};

use dioxus::prelude::*;

use crate::commands::palette::Command;

#[cfg(target_os = "macos")]
use objc2_foundation::{NSString, NSUserDefaults};

// Frame zero rendered with ascii.rest's original palettes at revision 9ba5ab14 (MIT).
const LIGHT_SCENES: [Scene; 3] = [
    Scene {
        id: "alpine-dawn",
        label: "Alpine Dawn",
        frame: asset!("/assets/ascii-rest/alpine-dawn.png"),
    },
    Scene {
        id: "cloud-sea",
        label: "Cloud Sea",
        frame: asset!("/assets/ascii-rest/cloud-sea.png"),
    },
    Scene {
        id: "salt-flat",
        label: "Salt Flat",
        frame: asset!("/assets/ascii-rest/salt-flat.png"),
    },
];

const DARK_SCENES: [Scene; 3] = [
    Scene {
        id: "night-coast",
        label: "Night Coast",
        frame: asset!("/assets/ascii-rest/night-coast.png"),
    },
    Scene {
        id: "desert-night",
        label: "Desert Night",
        frame: asset!("/assets/ascii-rest/desert-night.png"),
    },
    Scene {
        id: "aurora-fjord",
        label: "Aurora Fjord",
        frame: asset!("/assets/ascii-rest/aurora-fjord.png"),
    },
];

const RANDOM_ID: &str = "random";
static LAUNCH_SEED: OnceLock<RandomState> = OnceLock::new();

struct Scene {
    id: &'static str,
    label: &'static str,
    frame: Asset,
}

fn scenes(dark: bool) -> &'static [Scene] {
    if dark { &DARK_SCENES } else { &LIGHT_SCENES }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ScenePreferences {
    pinned: [Option<&'static str>; 2],
}

impl ScenePreferences {
    pub(crate) fn load() -> Self {
        let preferences = Self::default();
        #[cfg(target_os = "macos")]
        {
            let mut preferences = preferences;
            let defaults = NSUserDefaults::standardUserDefaults();
            for dark in [false, true] {
                if let Some(id) = defaults.stringForKey(&NSString::from_str(scene_key(dark)))
                    && !preferences.select(dark, &id.to_string())
                {
                    eprintln!("Could not restore landing scene: unknown value {id}");
                }
            }
            preferences
        }
        #[cfg(not(target_os = "macos"))]
        preferences
    }

    fn selection(self, dark: bool) -> &'static str {
        self.pinned[usize::from(dark)].unwrap_or(RANDOM_ID)
    }

    fn select(&mut self, dark: bool, id: &str) -> bool {
        let pinned = &mut self.pinned[usize::from(dark)];
        if id == RANDOM_ID {
            *pinned = None;
        } else if let Some(scene) = scenes(dark).iter().find(|scene| scene.id == id) {
            *pinned = Some(scene.id);
        } else {
            return false;
        }
        true
    }

    fn scene(self, dark: bool) -> &'static Scene {
        let scenes = scenes(dark);
        if let Some(id) = self.pinned[usize::from(dark)]
            && let Some(scene) = scenes.iter().find(|scene| scene.id == id)
        {
            return scene;
        }
        let index =
            LAUNCH_SEED.get_or_init(RandomState::new).hash_one(dark) as usize % scenes.len();
        &scenes[index]
    }
}

#[cfg(target_os = "macos")]
fn scene_key(dark: bool) -> &'static str {
    if dark {
        "xuy.dark-scene"
    } else {
        "xuy.light-scene"
    }
}

pub(crate) fn scene_commands(
    dark: bool,
    preferences: ScenePreferences,
) -> impl Iterator<Item = Command> {
    let selected = preferences.selection(dark);
    std::iter::once((RANDOM_ID, "Random"))
        .chain(scenes(dark).iter().map(|scene| (scene.id, scene.label)))
        .map(move |(id, label)| {
            Command::new(
                id,
                "Landing Scene",
                "home-01",
                "",
                "settings landing scene home background artwork",
            )
            .setting_value(label, id == selected)
        })
}

pub(crate) fn apply_scene_command(
    id: &str,
    dark: bool,
    mut preferences: Signal<ScenePreferences>,
) -> bool {
    let current = *preferences.peek();
    let mut next = current;
    if !next.select(dark, id) {
        return false;
    }
    if next == current {
        return true;
    }
    preferences.set(next);
    #[cfg(target_os = "macos")]
    {
        let defaults = NSUserDefaults::standardUserDefaults();
        let key = NSString::from_str(scene_key(dark));
        let id = next.selection(dark);
        if id == RANDOM_ID {
            defaults.removeObjectForKey(&key);
        } else {
            // A scene ID is a property-list string, as required by NSUserDefaults.
            unsafe { defaults.setObject_forKey(Some(&NSString::from_str(id)), &key) };
        }
    }
    true
}

#[component]
pub(crate) fn SceneSelect(dark: bool, preferences: Signal<ScenePreferences>) -> Element {
    let selected = preferences().selection(dark);
    rsx! {
        label {
            span { "Landing scene" }
            select {
                value: selected,
                onchange: move |event| {
                    apply_scene_command(&event.value(), dark, preferences);
                },
                option { value: RANDOM_ID, selected: selected == RANDOM_ID, "Random" }
                for scene in scenes(dark) {
                    option { key: "{scene.id}", value: scene.id, selected: selected == scene.id, "{scene.label}" }
                }
            }
        }
    }
}

#[component]
pub(crate) fn LandingScene(dark: bool, preferences: ScenePreferences) -> Element {
    let frame = preferences.scene(dark).frame;

    rsx! {
        document::Stylesheet { href: asset!("/assets/appearance.css") }
        div { class: "landing-scene", aria_hidden: true,
            "data-license": asset!("/assets/ascii-rest/LICENSE.txt"),
            img { src: frame, alt: "" }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{DARK_SCENES, LIGHT_SCENES, RANDOM_ID, ScenePreferences, scene_commands, scenes};

    #[test]
    fn themes_have_six_distinct_scenes() {
        let ids: HashSet<_> = LIGHT_SCENES
            .iter()
            .chain(DARK_SCENES.iter())
            .map(|scene| scene.id)
            .collect();
        assert_eq!(ids.len(), 6);
    }

    #[test]
    fn defaults_to_random_per_theme_and_keeps_that_scene_for_the_launch() {
        let preferences = ScenePreferences::default();
        for dark in [false, true] {
            assert_eq!(preferences.selection(dark), RANDOM_ID);
            let id = preferences.scene(dark).id;
            assert!(scenes(dark).iter().any(|scene| scene.id == id));
            assert_eq!(preferences.scene(dark).id, id);
        }
    }

    #[test]
    fn pinning_or_resetting_a_theme_preserves_the_other_theme() {
        let mut preferences = ScenePreferences::default();
        let light = &LIGHT_SCENES[1];
        let dark = &DARK_SCENES[0];
        preferences.select(false, light.id);
        assert_eq!(preferences.selection(true), RANDOM_ID);
        preferences.select(true, dark.id);
        assert!(!preferences.select(false, dark.id));
        assert_eq!(preferences.scene(false).id, light.id);
        assert_eq!(preferences.scene(true).id, dark.id);

        let mut restored = ScenePreferences::default();
        for dark in [false, true] {
            restored.select(dark, preferences.selection(dark));
        }
        assert!(restored == preferences);

        preferences.select(false, RANDOM_ID);
        assert_eq!(preferences.selection(false), RANDOM_ID);
        assert_eq!(preferences.selection(true), dark.id);
    }

    #[test]
    fn palette_shows_random_and_scenes_for_the_active_theme() {
        for dark in [false, true] {
            let choices: Vec<_> = scene_commands(dark, ScenePreferences::default())
                .map(|command| command.id)
                .collect();
            assert_eq!(choices[0], RANDOM_ID);
            assert!(
                choices[1..]
                    .iter()
                    .copied()
                    .eq(scenes(dark).iter().map(|scene| scene.id))
            );
        }
    }
}
