//! Scene Manager - Save and load scenes visually
//!
//! Handles exporting the current scene to YAML and loading scenes from files.
//! The user never needs to write YAML manually - this generates it from the visual state.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::editor::selection::Selectable;
use crate::physics::rigid_body::Mass;
use crate::scene::spawner::{ObjectSpawnConfig, SpawnShape, SpawnedObjects};

use super::spawn_palette::SpawnObjectEvent;
use super::EditorState;

/// Resource to track scene state
#[derive(Resource, Default)]
pub struct SceneManagerState {
    /// Current scene file path (if loaded/saved)
    pub current_file: Option<PathBuf>,
    /// Whether scene has unsaved changes
    pub has_unsaved_changes: bool,
    /// Scene name
    pub scene_name: String,
    /// Show save dialog
    pub show_save_dialog: bool,
    /// Show load dialog
    pub show_load_dialog: bool,
    /// Show new scene dialog
    pub show_new_dialog: bool,
    /// File path input for dialogs
    pub file_path_input: String,
    /// Error message to display
    pub error_message: Option<String>,
    /// Success message to display
    pub success_message: Option<String>,
}

impl SceneManagerState {
    pub fn new() -> Self {
        Self {
            scene_name: "Untitled".to_string(),
            file_path_input: "scene.yaml".to_string(),
            ..default()
        }
    }

    pub fn mark_changed(&mut self) {
        self.has_unsaved_changes = true;
    }

    pub fn mark_saved(&mut self, path: PathBuf) {
        self.has_unsaved_changes = false;
        self.current_file = Some(path);
    }
}

/// Serializable scene format
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializableScene {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub gravity: f32,
    pub objects: Vec<SerializableObject>,
    #[serde(default)]
    pub robots: Vec<SerializableRobot>,
    #[serde(default)]
    pub lighting: Option<SerializableLighting>,
}

impl Default for SerializableScene {
    fn default() -> Self {
        Self {
            name: "Untitled".to_string(),
            version: "1.0".to_string(),
            gravity: -9.81,
            objects: Vec::new(),
            robots: Vec::new(),
            lighting: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializableObject {
    pub name: String,
    pub shape: SerializableShape,
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation_euler: [f32; 3],
    #[serde(default)]
    pub is_static: bool,
    #[serde(default = "default_mass")]
    pub mass: f32,
    #[serde(default = "default_friction")]
    pub friction: f32,
    #[serde(default)]
    pub restitution: f32,
    #[serde(default = "default_color")]
    pub color: [f32; 4],
}

fn default_mass() -> f32 {
    1.0
}
fn default_friction() -> f32 {
    0.5
}
fn default_color() -> [f32; 4] {
    [0.8, 0.8, 0.8, 1.0]
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum SerializableShape {
    #[serde(rename = "box")]
    Box { size: [f32; 3] },
    #[serde(rename = "sphere")]
    Sphere { radius: f32 },
    #[serde(rename = "cylinder")]
    Cylinder { radius: f32, height: f32 },
    #[serde(rename = "capsule")]
    Capsule { radius: f32, height: f32 },
    #[serde(rename = "ground")]
    Ground { size_x: f32, size_z: f32 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializableRobot {
    pub name: String,
    pub urdf_path: String,
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation_euler: [f32; 3],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializableLighting {
    #[serde(default)]
    pub ambient_color: [f32; 3],
    #[serde(default = "default_ambient_brightness")]
    pub ambient_brightness: f32,
    #[serde(default)]
    pub directional_light: Option<SerializableDirectionalLight>,
}

fn default_ambient_brightness() -> f32 {
    0.3
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializableDirectionalLight {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub illuminance: f32,
}

/// Event to save current scene
#[derive(Event)]
pub struct SaveSceneEvent {
    pub path: PathBuf,
}

/// Event to load a scene
#[derive(Event)]
pub struct LoadSceneEvent {
    pub path: PathBuf,
}

/// Event to clear the current scene
#[derive(Event)]
pub struct ClearSceneEvent;

/// System to show the menu bar with File operations
pub fn menu_bar_system(
    mut contexts: EguiContexts,
    mut scene_state: ResMut<SceneManagerState>,
    mut save_events: EventWriter<SaveSceneEvent>,
    mut load_events: EventWriter<LoadSceneEvent>,
    mut clear_events: EventWriter<ClearSceneEvent>,
    state: Res<EditorState>,
) {
    if !state.enabled {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // File menu
            ui.menu_button("File", |ui| {
                if ui.button("New Scene").clicked() {
                    scene_state.show_new_dialog = true;
                    ui.close_menu();
                }

                if ui.button("Open...").clicked() {
                    scene_state.show_load_dialog = true;
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Save").clicked() {
                    if let Some(path) = scene_state.current_file.clone() {
                        save_events.send(SaveSceneEvent { path });
                    } else {
                        scene_state.show_save_dialog = true;
                    }
                    ui.close_menu();
                }

                if ui.button("Save As...").clicked() {
                    scene_state.show_save_dialog = true;
                    ui.close_menu();
                }
            });

            // Edit menu
            ui.menu_button("Edit", |ui| {
                if ui.button("Clear Scene").clicked() {
                    clear_events.send(ClearSceneEvent);
                    ui.close_menu();
                }
            });

            // Scene name and status
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if scene_state.has_unsaved_changes {
                    ui.label("*");
                }
                ui.label(&scene_state.scene_name);
            });
        });
    });

    // Save dialog
    if scene_state.show_save_dialog {
        show_save_dialog(ctx, &mut scene_state, &mut save_events);
    }

    // Load dialog
    if scene_state.show_load_dialog {
        show_load_dialog(ctx, &mut scene_state, &mut load_events);
    }

    // New scene dialog
    if scene_state.show_new_dialog {
        show_new_dialog(ctx, &mut scene_state, &mut clear_events);
    }

    // Show messages
    show_messages(ctx, &mut scene_state);
}

fn show_save_dialog(
    ctx: &egui::Context,
    scene_state: &mut SceneManagerState,
    save_events: &mut EventWriter<SaveSceneEvent>,
) {
    egui::Window::new("Save Scene")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("File name:");
                ui.text_edit_singleline(&mut scene_state.file_path_input);
            });

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    let path = PathBuf::from(&scene_state.file_path_input);
                    save_events.send(SaveSceneEvent { path });
                    scene_state.show_save_dialog = false;
                }
                if ui.button("Cancel").clicked() {
                    scene_state.show_save_dialog = false;
                }
            });
        });
}

fn show_load_dialog(
    ctx: &egui::Context,
    scene_state: &mut SceneManagerState,
    load_events: &mut EventWriter<LoadSceneEvent>,
) {
    egui::Window::new("Open Scene")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("File path:");
                ui.text_edit_singleline(&mut scene_state.file_path_input);
            });

            ui.horizontal(|ui| {
                if ui.button("Open").clicked() {
                    let path = PathBuf::from(&scene_state.file_path_input);
                    load_events.send(LoadSceneEvent { path });
                    scene_state.show_load_dialog = false;
                }
                if ui.button("Cancel").clicked() {
                    scene_state.show_load_dialog = false;
                }
            });
        });
}

fn show_new_dialog(
    ctx: &egui::Context,
    scene_state: &mut SceneManagerState,
    clear_events: &mut EventWriter<ClearSceneEvent>,
) {
    egui::Window::new("New Scene")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Create a new scene? Unsaved changes will be lost.");

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    clear_events.send(ClearSceneEvent);
                    scene_state.scene_name = "Untitled".to_string();
                    scene_state.current_file = None;
                    scene_state.has_unsaved_changes = false;
                    scene_state.show_new_dialog = false;
                }
                if ui.button("Cancel").clicked() {
                    scene_state.show_new_dialog = false;
                }
            });
        });
}

fn show_messages(ctx: &egui::Context, scene_state: &mut SceneManagerState) {
    // Error message
    if let Some(msg) = scene_state.error_message.clone() {
        egui::Window::new("Error")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.colored_label(egui::Color32::RED, msg);
                if ui.button("OK").clicked() {
                    scene_state.error_message = None;
                }
            });
    }

    // Success message
    if let Some(msg) = scene_state.success_message.clone() {
        egui::Window::new("Success")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.colored_label(egui::Color32::GREEN, msg);
                if ui.button("OK").clicked() {
                    scene_state.success_message = None;
                }
            });
    }
}

/// System to handle save events
pub fn save_scene_system(
    mut save_events: EventReader<SaveSceneEvent>,
    mut scene_state: ResMut<SceneManagerState>,
    query: Query<(
        Entity,
        &Name,
        &Transform,
        Option<&Mass>,
        Option<&Selectable>,
    )>,
    spawned_objects: Res<SpawnedObjects>,
) {
    for event in save_events.read() {
        let mut scene = SerializableScene {
            name: scene_state.scene_name.clone(),
            version: "1.0".to_string(),
            gravity: -9.81,
            objects: Vec::new(),
            robots: Vec::new(),
            lighting: None,
        };

        // Collect all spawned objects
        for entity in &spawned_objects.objects {
            if let Ok((_, name, transform, mass_opt, selectable_opt)) = query.get(*entity) {
                // Determine shape from name (simple heuristic)
                let shape = if name.as_str().contains("sphere") {
                    SerializableShape::Sphere { radius: 0.5 }
                } else if name.as_str().contains("cylinder") {
                    SerializableShape::Cylinder {
                        radius: 0.5,
                        height: 1.0,
                    }
                } else if name.as_str().contains("capsule") {
                    SerializableShape::Capsule {
                        radius: 0.3,
                        height: 1.0,
                    }
                } else if name.as_str().contains("ground") {
                    SerializableShape::Ground {
                        size_x: 20.0,
                        size_z: 20.0,
                    }
                } else {
                    SerializableShape::Box {
                        size: [1.0, 1.0, 1.0],
                    }
                };

                let (x, y, z) = transform.rotation.to_euler(EulerRot::XYZ);

                let obj = SerializableObject {
                    name: name.as_str().to_string(),
                    shape,
                    position: [
                        transform.translation.x,
                        transform.translation.y,
                        transform.translation.z,
                    ],
                    rotation_euler: [x.to_degrees(), y.to_degrees(), z.to_degrees()],
                    is_static: name.as_str().contains("ground"),
                    mass: mass_opt.map(|m| m.mass).unwrap_or(1.0),
                    friction: 0.5,
                    restitution: 0.3,
                    color: [0.8, 0.4, 0.2, 1.0],
                };

                scene.objects.push(obj);
            }
        }

        // Write to file
        match serde_yaml::to_string(&scene) {
            Ok(yaml) => {
                let yaml_with_header = format!(
                    "# sim3d Scene File\n# Generated by sim3d visual editor\n# Do not edit manually unless you know what you're doing\n\n{}",
                    yaml
                );

                match std::fs::write(&event.path, yaml_with_header) {
                    Ok(_) => {
                        scene_state.mark_saved(event.path.clone());
                        scene_state.success_message =
                            Some(format!("Scene saved to {:?}", event.path));
                        info!("Scene saved to {:?}", event.path);
                    }
                    Err(e) => {
                        scene_state.error_message = Some(format!("Failed to save: {}", e));
                        error!("Failed to save scene: {}", e);
                    }
                }
            }
            Err(e) => {
                scene_state.error_message = Some(format!("Failed to serialize: {}", e));
                error!("Failed to serialize scene: {}", e);
            }
        }
    }
}

/// System to handle load events
pub fn load_scene_system(
    mut load_events: EventReader<LoadSceneEvent>,
    mut scene_state: ResMut<SceneManagerState>,
    mut clear_events: EventWriter<ClearSceneEvent>,
    mut spawn_events: EventWriter<SpawnObjectEvent>,
) {
    for event in load_events.read() {
        match std::fs::read_to_string(&event.path) {
            Ok(yaml) => match serde_yaml::from_str::<SerializableScene>(&yaml) {
                Ok(scene) => {
                    // Clear existing scene
                    clear_events.send(ClearSceneEvent);

                    // Spawn all objects from the scene
                    for obj in &scene.objects {
                        let shape = match &obj.shape {
                            SerializableShape::Box { size } => SpawnShape::Box {
                                size: Vec3::new(size[0], size[1], size[2]),
                            },
                            SerializableShape::Sphere { radius } => {
                                SpawnShape::Sphere { radius: *radius }
                            }
                            SerializableShape::Cylinder { radius, height } => {
                                SpawnShape::Cylinder {
                                    radius: *radius,
                                    height: *height,
                                }
                            }
                            SerializableShape::Capsule { radius, height } => SpawnShape::Capsule {
                                radius: *radius,
                                height: *height,
                            },
                            SerializableShape::Ground { size_x, size_z } => SpawnShape::Ground {
                                size_x: *size_x,
                                size_z: *size_z,
                            },
                        };

                        let mut config = ObjectSpawnConfig::new(obj.name.clone(), shape)
                            .at_position(Vec3::new(
                                obj.position[0],
                                obj.position[1],
                                obj.position[2],
                            ))
                            .with_rotation_euler(
                                obj.rotation_euler[0].to_radians(),
                                obj.rotation_euler[1].to_radians(),
                                obj.rotation_euler[2].to_radians(),
                            )
                            .with_mass(obj.mass)
                            .with_friction(obj.friction)
                            .with_restitution(obj.restitution)
                            .with_color(Color::srgba(
                                obj.color[0],
                                obj.color[1],
                                obj.color[2],
                                obj.color[3],
                            ));

                        if obj.is_static {
                            config = config.as_static();
                        }

                        spawn_events.send(SpawnObjectEvent { config });
                    }

                    scene_state.scene_name = scene.name;
                    scene_state.current_file = Some(event.path.clone());
                    scene_state.has_unsaved_changes = false;
                    scene_state.success_message =
                        Some(format!("Scene loaded from {:?}", event.path));
                    info!("Scene loaded from {:?}", event.path);
                }
                Err(e) => {
                    scene_state.error_message = Some(format!("Invalid scene file: {}", e));
                    error!("Failed to parse scene: {}", e);
                }
            },
            Err(e) => {
                scene_state.error_message = Some(format!("Failed to read file: {}", e));
                error!("Failed to read scene file: {}", e);
            }
        }
    }
}

/// System to handle clear scene events
pub fn clear_scene_system(
    mut commands: Commands,
    mut clear_events: EventReader<ClearSceneEvent>,
    mut spawned_objects: ResMut<SpawnedObjects>,
    mut scene_state: ResMut<SceneManagerState>,
) {
    for _ in clear_events.read() {
        // Despawn all spawned objects
        for entity in spawned_objects.objects.drain(..) {
            commands.entity(entity).despawn_recursive();
        }

        scene_state.has_unsaved_changes = true;
        info!("Scene cleared");
    }
}

/// Plugin for scene management
pub struct SceneManagerPlugin;

impl Plugin for SceneManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneManagerState>()
            .add_event::<SaveSceneEvent>()
            .add_event::<LoadSceneEvent>()
            .add_event::<ClearSceneEvent>()
            .add_systems(
                Update,
                (
                    menu_bar_system,
                    save_scene_system,
                    load_scene_system,
                    clear_scene_system,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializable_scene() {
        let scene = SerializableScene {
            name: "Test".to_string(),
            version: "1.0".to_string(),
            gravity: -9.81,
            objects: vec![SerializableObject {
                name: "box_1".to_string(),
                shape: SerializableShape::Box {
                    size: [1.0, 1.0, 1.0],
                },
                position: [0.0, 1.0, 0.0],
                rotation_euler: [0.0, 0.0, 0.0],
                is_static: false,
                mass: 1.0,
                friction: 0.5,
                restitution: 0.3,
                color: [0.8, 0.4, 0.2, 1.0],
            }],
            robots: Vec::new(),
            lighting: None,
        };

        let yaml = serde_yaml::to_string(&scene).unwrap();
        assert!(yaml.contains("box_1"));
        assert!(yaml.contains("type: box"));
    }

    #[test]
    fn test_scene_deserialization() {
        let yaml = r#"
name: Test
version: "1.0"
gravity: -9.81
objects:
  - name: ground_1
    shape:
      type: ground
      size_x: 20.0
      size_z: 20.0
    position: [0.0, 0.0, 0.0]
    is_static: true
robots: []
"#;

        let scene: SerializableScene = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(scene.name, "Test");
        assert_eq!(scene.objects.len(), 1);
        assert_eq!(scene.objects[0].name, "ground_1");
    }
}
