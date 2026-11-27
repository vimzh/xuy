//! Home artwork and its theme-specific scene preferences.

mod landing;

pub(crate) use landing::{
    LandingScene, ScenePreferences, SceneSelect, apply_scene_command, scene_commands,
};
