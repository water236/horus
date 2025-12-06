//! Robot Browser - Visual URDF/Robot loader
//!
//! Provides a visual interface for browsing, previewing, and loading robots
//! from URDF files or built-in presets.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::path::PathBuf;

use super::EditorState;

/// Resource to track robot browser state
#[derive(Resource)]
pub struct RobotBrowserState {
    /// Whether the browser window is open
    pub show_browser: bool,
    /// Current tab (Presets or Custom)
    pub current_tab: RobotBrowserTab,
    /// Custom URDF path input
    pub custom_urdf_path: String,
    /// Selected preset
    pub selected_preset: Option<RobotPreset>,
    /// Spawn position
    pub spawn_position: [f32; 3],
    /// Robot name override
    pub robot_name: String,
    /// Error message
    pub error_message: Option<String>,
    /// Success message
    pub success_message: Option<String>,
}

impl Default for RobotBrowserState {
    fn default() -> Self {
        Self {
            show_browser: false,
            current_tab: RobotBrowserTab::Presets,
            custom_urdf_path: String::new(),
            selected_preset: None,
            spawn_position: [0.0, 0.1, 0.0],
            robot_name: "robot".to_string(),
            error_message: None,
            success_message: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RobotBrowserTab {
    Presets,
    Custom,
    Recent,
}

/// Robot preset with metadata
#[derive(Clone, Debug)]
pub struct RobotPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub urdf_package: &'static str,
    pub urdf_path: &'static str,
    pub default_height: f32,
}

/// List of built-in robot presets
pub const ROBOT_PRESETS: &[RobotPreset] = &[
    RobotPreset {
        name: "TurtleBot3 Burger",
        description: "Small differential drive robot for education and research",
        category: "Mobile",
        urdf_package: "turtlebot3_description",
        urdf_path: "urdf/turtlebot3_burger.urdf",
        default_height: 0.1,
    },
    RobotPreset {
        name: "TurtleBot3 Waffle",
        description: "Larger TurtleBot3 variant with more sensors",
        category: "Mobile",
        urdf_package: "turtlebot3_description",
        urdf_path: "urdf/turtlebot3_waffle.urdf",
        default_height: 0.1,
    },
    RobotPreset {
        name: "UR5",
        description: "Universal Robots 5kg payload arm",
        category: "Arm",
        urdf_package: "ur_description",
        urdf_path: "urdf/ur5.urdf",
        default_height: 0.0,
    },
    RobotPreset {
        name: "UR10",
        description: "Universal Robots 10kg payload arm",
        category: "Arm",
        urdf_package: "ur_description",
        urdf_path: "urdf/ur10.urdf",
        default_height: 0.0,
    },
    RobotPreset {
        name: "Franka Panda",
        description: "7-DOF collaborative robot arm",
        category: "Arm",
        urdf_package: "franka_description",
        urdf_path: "robots/panda/panda.urdf",
        default_height: 0.0,
    },
    RobotPreset {
        name: "Fetch",
        description: "Mobile manipulation robot",
        category: "Mobile Manipulator",
        urdf_package: "fetch_description",
        urdf_path: "robots/fetch.urdf",
        default_height: 0.1,
    },
    RobotPreset {
        name: "Spot",
        description: "Boston Dynamics quadruped robot",
        category: "Legged",
        urdf_package: "spot_description",
        urdf_path: "urdf/spot.urdf",
        default_height: 0.5,
    },
    RobotPreset {
        name: "ANYmal",
        description: "ANYbotics quadruped robot",
        category: "Legged",
        urdf_package: "anymal_description",
        urdf_path: "urdf/anymal.urdf",
        default_height: 0.5,
    },
    RobotPreset {
        name: "Husky",
        description: "Clearpath Husky mobile robot",
        category: "Mobile",
        urdf_package: "husky_description",
        urdf_path: "urdf/husky.urdf",
        default_height: 0.15,
    },
    RobotPreset {
        name: "Jackal",
        description: "Clearpath Jackal mobile robot",
        category: "Mobile",
        urdf_package: "jackal_description",
        urdf_path: "urdf/jackal.urdf",
        default_height: 0.1,
    },
];

/// Event to load a robot
#[derive(Event, Clone)]
pub struct LoadRobotEvent {
    pub urdf_path: String,
    pub name: String,
    pub position: Vec3,
}

/// System to show robot browser button in spawn palette
/// Note: This is a placeholder - the actual button is integrated into spawn_palette.rs
#[allow(dead_code)]
pub fn robot_button_system(
    _contexts: EguiContexts,
    _browser_state: ResMut<RobotBrowserState>,
    _state: Res<EditorState>,
) {
    // This is a placeholder system
    // The actual "Load Robot" button is integrated into spawn_palette.rs
}

/// System to show robot browser window
pub fn robot_browser_system(
    mut contexts: EguiContexts,
    mut browser_state: ResMut<RobotBrowserState>,
    mut load_events: EventWriter<LoadRobotEvent>,
    state: Res<EditorState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !state.enabled {
        return;
    }

    // Toggle with Ctrl+R
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::KeyR) {
        browser_state.show_browser = !browser_state.show_browser;
    }

    if !browser_state.show_browser {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::Window::new("Robot Browser")
        .default_width(500.0)
        .default_height(400.0)
        .collapsible(true)
        .resizable(true)
        .show(ctx, |ui| {
            // Tab bar
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        browser_state.current_tab == RobotBrowserTab::Presets,
                        "Presets",
                    )
                    .clicked()
                {
                    browser_state.current_tab = RobotBrowserTab::Presets;
                }
                if ui
                    .selectable_label(
                        browser_state.current_tab == RobotBrowserTab::Custom,
                        "Custom URDF",
                    )
                    .clicked()
                {
                    browser_state.current_tab = RobotBrowserTab::Custom;
                }
            });

            ui.separator();

            match browser_state.current_tab {
                RobotBrowserTab::Presets => {
                    show_presets_tab(ui, &mut browser_state, &mut load_events);
                }
                RobotBrowserTab::Custom => {
                    show_custom_tab(ui, &mut browser_state, &mut load_events);
                }
                RobotBrowserTab::Recent => {
                    ui.label("Recent robots will appear here");
                }
            }

            // Messages
            if let Some(msg) = &browser_state.error_message {
                ui.separator();
                ui.colored_label(egui::Color32::RED, msg);
            }
            if let Some(msg) = &browser_state.success_message {
                ui.separator();
                ui.colored_label(egui::Color32::GREEN, msg);
            }
        });
}

fn show_presets_tab(
    ui: &mut egui::Ui,
    browser_state: &mut RobotBrowserState,
    load_events: &mut EventWriter<LoadRobotEvent>,
) {
    // Group presets by category
    let categories = ["Mobile", "Arm", "Mobile Manipulator", "Legged"];

    egui::ScrollArea::vertical().show(ui, |ui| {
        for category in categories {
            ui.collapsing(category, |ui| {
                for preset in ROBOT_PRESETS.iter().filter(|p| p.category == category) {
                    ui.horizontal(|ui| {
                        let is_selected = browser_state
                            .selected_preset
                            .as_ref()
                            .map(|p| p.name == preset.name)
                            .unwrap_or(false);

                        if ui.selectable_label(is_selected, preset.name).clicked() {
                            browser_state.selected_preset = Some(preset.clone());
                            browser_state.robot_name = preset.name.replace(" ", "_").to_lowercase();
                            browser_state.spawn_position[1] = preset.default_height;
                        }

                        ui.label("|");
                        ui.label(preset.description);
                    });
                }
            });
        }

        ui.separator();

        // Selected preset details
        if let Some(preset) = &browser_state.selected_preset {
            ui.group(|ui| {
                ui.heading(preset.name);
                ui.label(preset.description);
                ui.label(format!("Category: {}", preset.category));
                ui.label(format!("Package: {}", preset.urdf_package));

                ui.separator();

                // Spawn settings
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut browser_state.robot_name);
                });

                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.add(
                        egui::DragValue::new(&mut browser_state.spawn_position[0]).prefix("X: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut browser_state.spawn_position[1]).prefix("Y: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut browser_state.spawn_position[2]).prefix("Z: "),
                    );
                });

                ui.separator();

                if ui.button("Load Robot").clicked() {
                    let urdf_path = format!("{}/{}", preset.urdf_package, preset.urdf_path);
                    load_events.send(LoadRobotEvent {
                        urdf_path,
                        name: browser_state.robot_name.clone(),
                        position: Vec3::new(
                            browser_state.spawn_position[0],
                            browser_state.spawn_position[1],
                            browser_state.spawn_position[2],
                        ),
                    });
                    browser_state.success_message = Some(format!("Loading robot: {}", preset.name));
                }
            });
        } else {
            ui.label("Select a robot preset to see details");
        }
    });
}

fn show_custom_tab(
    ui: &mut egui::Ui,
    browser_state: &mut RobotBrowserState,
    load_events: &mut EventWriter<LoadRobotEvent>,
) {
    ui.label("Load a custom URDF file:");

    ui.horizontal(|ui| {
        ui.label("URDF Path:");
        ui.text_edit_singleline(&mut browser_state.custom_urdf_path);

        #[cfg(feature = "visual")]
        if ui.button("Browse...").clicked() {
            // Use native file dialog if rfd is available
            #[cfg(feature = "visual")]
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("URDF", &["urdf", "xacro", "xml"])
                    .pick_file()
                {
                    browser_state.custom_urdf_path = path.to_string_lossy().to_string();
                }
            }
        }
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Robot Name:");
        ui.text_edit_singleline(&mut browser_state.robot_name);
    });

    ui.horizontal(|ui| {
        ui.label("Position:");
        ui.add(egui::DragValue::new(&mut browser_state.spawn_position[0]).prefix("X: "));
        ui.add(egui::DragValue::new(&mut browser_state.spawn_position[1]).prefix("Y: "));
        ui.add(egui::DragValue::new(&mut browser_state.spawn_position[2]).prefix("Z: "));
    });

    ui.separator();

    if ui.button("Load URDF").clicked() {
        if browser_state.custom_urdf_path.is_empty() {
            browser_state.error_message = Some("Please specify a URDF path".to_string());
        } else {
            let path = PathBuf::from(&browser_state.custom_urdf_path);
            if path.exists() || browser_state.custom_urdf_path.contains("://") {
                load_events.send(LoadRobotEvent {
                    urdf_path: browser_state.custom_urdf_path.clone(),
                    name: browser_state.robot_name.clone(),
                    position: Vec3::new(
                        browser_state.spawn_position[0],
                        browser_state.spawn_position[1],
                        browser_state.spawn_position[2],
                    ),
                });
                browser_state.success_message =
                    Some(format!("Loading: {}", browser_state.custom_urdf_path));
                browser_state.error_message = None;
            } else {
                browser_state.error_message = Some(format!("File not found: {}", path.display()));
            }
        }
    }

    ui.separator();

    // Help text
    ui.collapsing("Help", |ui| {
        ui.label("Supported formats:");
        ui.label("  • URDF (.urdf) - Unified Robot Description Format");
        ui.label("  • Xacro (.xacro) - XML macro files (requires xacro installed)");
        ui.label("");
        ui.label("Example paths:");
        ui.label("  • /path/to/robot.urdf");
        ui.label("  • package://my_robot/urdf/robot.urdf");
    });
}

/// System to handle robot loading
pub fn process_load_robot_system(
    mut load_events: EventReader<LoadRobotEvent>,
    mut browser_state: ResMut<RobotBrowserState>,
) {
    for event in load_events.read() {
        info!(
            "Loading robot '{}' from {} at {:?}",
            event.name, event.urdf_path, event.position
        );

        // TODO: Integrate with actual URDF loader
        // This would call into robot/urdf_loader.rs

        browser_state.success_message = Some(format!(
            "Robot '{}' loaded at position ({:.1}, {:.1}, {:.1})",
            event.name, event.position.x, event.position.y, event.position.z
        ));
    }
}

/// Plugin for robot browser
pub struct RobotBrowserPlugin;

impl Plugin for RobotBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RobotBrowserState>()
            .add_event::<LoadRobotEvent>()
            .add_systems(Update, (robot_browser_system, process_load_robot_system));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robot_presets() {
        assert!(!ROBOT_PRESETS.is_empty());

        // Check that all presets have required fields
        for preset in ROBOT_PRESETS {
            assert!(!preset.name.is_empty());
            assert!(!preset.description.is_empty());
            assert!(!preset.category.is_empty());
            assert!(!preset.urdf_package.is_empty());
            assert!(!preset.urdf_path.is_empty());
        }
    }

    #[test]
    fn test_browser_state_default() {
        let state = RobotBrowserState::default();
        assert!(!state.show_browser);
        assert_eq!(state.current_tab, RobotBrowserTab::Presets);
    }
}
