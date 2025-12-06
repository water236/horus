//! Visual Editor Plugin - Standalone visual-first UX for sim3d
//!
//! This plugin provides the new visual-first editing experience without
//! depending on the legacy editor code that has Bevy 0.15 compatibility issues.
//!
//! Features:
//! - Click-to-spawn objects from the palette
//! - Property editor with sliders
//! - Robot browser with presets
//! - Scene save/load (auto-generates YAML)

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use super::robot_browser::{LoadRobotEvent, RobotBrowserState};
use super::scene_manager::{ClearSceneEvent, LoadSceneEvent, SaveSceneEvent, SceneManagerState};
use super::spawn_palette::{SpawnObjectEvent, SpawnPaletteState, SpawnRobotEvent};

/// Simplified editor state for the visual editor
#[derive(Resource)]
pub struct VisualEditorState {
    /// Whether the editor is enabled
    pub enabled: bool,
    /// Show the property panel
    pub show_properties: bool,
}

impl Default for VisualEditorState {
    fn default() -> Self {
        Self {
            enabled: true,
            show_properties: true,
        }
    }
}

/// Visual Editor Plugin - provides visual-first UX without legacy dependencies
///
/// This is the recommended plugin for the new visual editing experience.
/// It provides:
/// - Spawn palette at the bottom for adding objects
/// - Property editor on the right for editing selected objects
/// - Robot browser for loading URDF robots
/// - Scene save/load functionality
pub struct VisualEditorPlugin;

impl Plugin for VisualEditorPlugin {
    fn build(&self, app: &mut App) {
        // Add egui if not already present
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        app
            // Resources
            .init_resource::<VisualEditorState>()
            .init_resource::<SpawnPaletteState>()
            .init_resource::<SceneManagerState>()
            .init_resource::<RobotBrowserState>()
            // Events
            .add_event::<SpawnObjectEvent>()
            .add_event::<SpawnRobotEvent>()
            .add_event::<SaveSceneEvent>()
            .add_event::<LoadSceneEvent>()
            .add_event::<ClearSceneEvent>()
            .add_event::<LoadRobotEvent>()
            // Systems
            .add_systems(
                Update,
                (
                    // Menu bar
                    super::scene_manager::menu_bar_system,
                    // Spawn palette
                    super::spawn_palette::spawn_palette_panel_system,
                    super::spawn_palette::quick_spawn_menu_system,
                    super::spawn_palette::process_spawn_events_system,
                    // Robot browser
                    super::robot_browser::robot_browser_system,
                    super::robot_browser::process_load_robot_system,
                    // Scene operations
                    super::scene_manager::save_scene_system,
                    super::scene_manager::load_scene_system,
                    super::scene_manager::clear_scene_system,
                    // Property editor
                    super::property_editor::property_editor_panel_system,
                )
                    .run_if(visual_editor_enabled),
            );
    }
}

fn visual_editor_enabled(state: Res<VisualEditorState>) -> bool {
    state.enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_editor_state_default() {
        let state = VisualEditorState::default();
        assert!(state.enabled);
        assert!(state.show_properties);
    }
}
