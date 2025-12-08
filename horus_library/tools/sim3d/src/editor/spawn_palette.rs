//! Spawn Palette - Visual UI for adding objects to the scene
//!
//! Provides a clickable palette of objects that users can add to the scene
//! without writing any code or YAML.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::physics::world::PhysicsWorld;
use crate::scene::spawner::{ObjectSpawnConfig, ObjectSpawner, SpawnShape, SpawnedObjects};

use super::selection::{Selectable, Selection};
use super::EditorState;

/// Resource to track spawn palette state
#[derive(Resource, Default)]
pub struct SpawnPaletteState {
    /// Whether the palette is open
    pub show_palette: bool,
    /// Whether the quick menu is open (Space key)
    pub show_quick_menu: bool,
    /// Search filter for quick menu
    pub search_filter: String,
    /// Next object counter for unique names
    pub object_counter: u32,
    /// Pending spawn (set when user clicks, spawned next frame with physics access)
    pub pending_spawn: Option<PendingSpawn>,
    /// Default spawn height above ground
    pub spawn_height: f32,
    /// Default physics properties for new objects
    pub default_mass: f32,
    pub default_friction: f32,
    pub default_restitution: f32,
    /// Color picker state
    pub selected_color: [f32; 3],
}

impl SpawnPaletteState {
    pub fn new() -> Self {
        Self {
            show_palette: true,
            show_quick_menu: false,
            search_filter: String::new(),
            object_counter: 0,
            pending_spawn: None,
            spawn_height: 1.0,
            default_mass: 1.0,
            default_friction: 0.5,
            default_restitution: 0.3,
            selected_color: [0.8, 0.4, 0.2], // Orange default
        }
    }

    pub fn next_name(&mut self, prefix: &str) -> String {
        self.object_counter += 1;
        format!("{}_{}", prefix, self.object_counter)
    }
}

/// Represents an object waiting to be spawned
#[derive(Clone)]
pub struct PendingSpawn {
    pub config: ObjectSpawnConfig,
}

/// Event sent when user wants to spawn an object
#[derive(Event, Clone)]
pub struct SpawnObjectEvent {
    pub config: ObjectSpawnConfig,
}

/// Event sent when user wants to spawn a robot
#[derive(Event, Clone)]
pub struct SpawnRobotEvent {
    pub urdf_path: String,
    pub position: Vec3,
    pub name: String,
}

/// System to show the spawn palette panel
pub fn spawn_palette_panel_system(
    mut contexts: EguiContexts,
    state: Res<EditorState>,
    mut palette_state: ResMut<SpawnPaletteState>,
    mut spawn_events: EventWriter<SpawnObjectEvent>,
) {
    if !state.enabled {
        return;
    }

    let ctx = contexts.ctx_mut();

    // Bottom panel with spawn buttons
    egui::TopBottomPanel::bottom("spawn_palette")
        .resizable(false)
        .min_height(80.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Create");
                ui.separator();

                // Primitive shapes
                spawn_button(ui, "Box", &mut palette_state, &mut spawn_events, || {
                    SpawnShape::Box { size: Vec3::ONE }
                });

                spawn_button(ui, "Sphere", &mut palette_state, &mut spawn_events, || {
                    SpawnShape::Sphere { radius: 0.5 }
                });

                spawn_button(
                    ui,
                    "Cylinder",
                    &mut palette_state,
                    &mut spawn_events,
                    || SpawnShape::Cylinder {
                        radius: 0.5,
                        height: 1.0,
                    },
                );

                spawn_button(ui, "Capsule", &mut palette_state, &mut spawn_events, || {
                    SpawnShape::Capsule {
                        radius: 0.3,
                        height: 1.0,
                    }
                });

                ui.separator();

                // Ground plane
                if ui
                    .button("Ground")
                    .on_hover_text("Add a ground plane")
                    .clicked()
                {
                    let config = ObjectSpawnConfig::new(
                        palette_state.next_name("ground"),
                        SpawnShape::Ground {
                            size_x: 20.0,
                            size_z: 20.0,
                        },
                    )
                    .at_position(Vec3::ZERO)
                    .as_static()
                    .with_color(Color::srgb(0.3, 0.5, 0.3))
                    .with_friction(0.8);

                    spawn_events.send(SpawnObjectEvent { config });
                }

                ui.separator();

                // Properties for new objects
                ui.vertical(|ui| {
                    ui.label("Properties:");
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        ui.add(
                            egui::DragValue::new(&mut palette_state.spawn_height)
                                .speed(0.1)
                                .range(0.0..=20.0),
                        );
                    });
                });

                ui.separator();

                // Color picker
                ui.vertical(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_rgb(&mut palette_state.selected_color);
                });
            });
        });
}

/// Helper function to create a spawn button
fn spawn_button<F>(
    ui: &mut egui::Ui,
    label: &str,
    palette_state: &mut SpawnPaletteState,
    spawn_events: &mut EventWriter<SpawnObjectEvent>,
    shape_fn: F,
) where
    F: FnOnce() -> SpawnShape,
{
    if ui.button(label).clicked() {
        let shape = shape_fn();
        let name = palette_state.next_name(&label.to_lowercase());
        let color = Color::srgb(
            palette_state.selected_color[0],
            palette_state.selected_color[1],
            palette_state.selected_color[2],
        );

        let config = ObjectSpawnConfig::new(name, shape)
            .at_position(Vec3::new(0.0, palette_state.spawn_height, 0.0))
            .with_mass(palette_state.default_mass)
            .with_friction(palette_state.default_friction)
            .with_restitution(palette_state.default_restitution)
            .with_color(color);

        spawn_events.send(SpawnObjectEvent { config });
    }
}

/// System to show quick spawn menu (Space key)
pub fn quick_spawn_menu_system(
    mut contexts: EguiContexts,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut palette_state: ResMut<SpawnPaletteState>,
    mut spawn_events: EventWriter<SpawnObjectEvent>,
    state: Res<EditorState>,
) {
    if !state.enabled {
        return;
    }

    // Toggle quick menu with Space
    if keyboard.just_pressed(KeyCode::Space) {
        palette_state.show_quick_menu = !palette_state.show_quick_menu;
        palette_state.search_filter.clear();
    }

    // Close with Escape
    if keyboard.just_pressed(KeyCode::Escape) {
        palette_state.show_quick_menu = false;
    }

    if !palette_state.show_quick_menu {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::Window::new("Quick Add")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Search box
            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.text_edit_singleline(&mut palette_state.search_filter);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Spawn first matching item
                    palette_state.show_quick_menu = false;
                }
                response.request_focus();
            });

            ui.separator();

            let filter = palette_state.search_filter.to_lowercase();

            // List of spawnable items
            let items = [
                ("Box", "box", SpawnShape::Box { size: Vec3::ONE }),
                ("Sphere", "sphere ball", SpawnShape::Sphere { radius: 0.5 }),
                (
                    "Cylinder",
                    "cylinder tube",
                    SpawnShape::Cylinder {
                        radius: 0.5,
                        height: 1.0,
                    },
                ),
                (
                    "Capsule",
                    "capsule pill",
                    SpawnShape::Capsule {
                        radius: 0.3,
                        height: 1.0,
                    },
                ),
                (
                    "Ground",
                    "ground floor plane",
                    SpawnShape::Ground {
                        size_x: 20.0,
                        size_z: 20.0,
                    },
                ),
                (
                    "Wall",
                    "wall barrier",
                    SpawnShape::Box {
                        size: Vec3::new(5.0, 2.0, 0.2),
                    },
                ),
                (
                    "Ramp",
                    "ramp slope incline",
                    SpawnShape::Box {
                        size: Vec3::new(3.0, 0.2, 2.0),
                    },
                ),
                (
                    "Pillar",
                    "pillar column post",
                    SpawnShape::Cylinder {
                        radius: 0.2,
                        height: 3.0,
                    },
                ),
            ];

            for (name, keywords, shape) in items {
                // Filter check and click handler are intentionally separate:
                // - First if: controls whether button is rendered
                // - Second if: handles click event
                #[allow(clippy::collapsible_if)]
                if filter.is_empty()
                    || name.to_lowercase().contains(&filter)
                    || keywords.contains(filter.as_str())
                {
                    if ui.button(name).clicked() {
                        let obj_name = palette_state.next_name(&name.to_lowercase());
                        let is_static = matches!(shape, SpawnShape::Ground { .. });
                        let color = Color::srgb(
                            palette_state.selected_color[0],
                            palette_state.selected_color[1],
                            palette_state.selected_color[2],
                        );

                        let mut config = ObjectSpawnConfig::new(obj_name, shape)
                            .at_position(Vec3::new(0.0, palette_state.spawn_height, 0.0))
                            .with_mass(palette_state.default_mass)
                            .with_friction(palette_state.default_friction)
                            .with_restitution(palette_state.default_restitution)
                            .with_color(color);

                        if is_static {
                            config = config.as_static();
                        }

                        // Special handling for ramp
                        if name == "Ramp" {
                            config = config.with_rotation_euler(0.3, 0.0, 0.0); // Slight tilt
                        }

                        spawn_events.send(SpawnObjectEvent { config });
                        palette_state.show_quick_menu = false;
                    }
                }
            }

            ui.separator();
            ui.label("Press Space to close, Enter to select first");
        });
}

/// System to process spawn events and create actual objects
pub fn process_spawn_events_system(
    mut commands: Commands,
    mut spawn_events: EventReader<SpawnObjectEvent>,
    mut physics_world: ResMut<PhysicsWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned_objects: ResMut<SpawnedObjects>,
    mut selection: ResMut<Selection>,
) {
    for event in spawn_events.read() {
        let config = event.config.clone();
        let name = config.name.clone();

        // Spawn the object using existing ObjectSpawner
        let entity = ObjectSpawner::spawn_object(
            config,
            &mut commands,
            &mut physics_world,
            &mut meshes,
            &mut materials,
        );

        // Add Selectable component for editor interaction
        commands.entity(entity).insert(Selectable::new(&name));

        // Track spawned object
        spawned_objects.add(entity);

        // Auto-select newly spawned object
        selection.select(entity);

        info!("Spawned object: {} (entity {:?})", name, entity);
    }
}

/// Plugin to add spawn palette functionality
pub struct SpawnPalettePlugin;

impl Plugin for SpawnPalettePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnPaletteState>()
            .add_event::<SpawnObjectEvent>()
            .add_event::<SpawnRobotEvent>()
            .add_systems(
                Update,
                (
                    spawn_palette_panel_system,
                    quick_spawn_menu_system,
                    process_spawn_events_system,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_palette_state() {
        let mut state = SpawnPaletteState::new();
        assert_eq!(state.next_name("box"), "box_1");
        assert_eq!(state.next_name("box"), "box_2");
        assert_eq!(state.next_name("sphere"), "sphere_3");
    }

    #[test]
    fn test_spawn_config_creation() {
        let config = ObjectSpawnConfig::new("test", SpawnShape::Box { size: Vec3::ONE })
            .at_position(Vec3::new(1.0, 2.0, 3.0))
            .with_mass(5.0)
            .with_friction(0.8);

        assert_eq!(config.name, "test");
        assert_eq!(config.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(config.mass, 5.0);
        assert_eq!(config.friction, 0.8);
    }
}
