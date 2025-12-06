//! Property Editor - Editable sliders for object properties
//!
//! Provides visual editing of physics properties (mass, friction, restitution)
//! and visual properties (color, material) for selected objects.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// Physics types for future integration
#[allow(unused_imports)]
use crate::physics::rigid_body::{Damping, Mass, Velocity};

use super::scene_manager::SceneManagerState;
use super::selection::Selection;
use super::EditorState;

/// Component to store editable physics properties
#[derive(Component, Clone, Debug)]
pub struct EditablePhysics {
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub is_static: bool,
}

impl Default for EditablePhysics {
    fn default() -> Self {
        Self {
            mass: 1.0,
            friction: 0.5,
            restitution: 0.3,
            linear_damping: 0.1,
            angular_damping: 0.1,
            is_static: false,
        }
    }
}

/// Component to store editable visual properties
#[derive(Component, Clone, Debug)]
pub struct EditableVisual {
    pub color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
}

impl Default for EditableVisual {
    fn default() -> Self {
        Self {
            color: [0.8, 0.4, 0.2, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
        }
    }
}

/// System to show property editor panel with sliders
pub fn property_editor_panel_system(
    mut contexts: EguiContexts,
    state: Res<EditorState>,
    selection: Res<Selection>,
    mut scene_state: ResMut<SceneManagerState>,
    mut transforms: Query<&mut Transform>,
    mut physics_query: Query<&mut EditablePhysics>,
    mut visual_query: Query<&mut EditableVisual>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    material_handles: Query<&MeshMaterial3d<StandardMaterial>>,
    names: Query<&Name>,
) {
    if !state.enabled || !state.show_inspector {
        return;
    }

    let Some(entity) = selection.primary else {
        return;
    };

    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("property_editor")
        .default_width(320.0)
        .show(ctx, |ui| {
            ui.heading("Properties");
            ui.separator();

            // Entity info
            if let Ok(name) = names.get(entity) {
                ui.label(format!("Selected: {}", name.as_str()));
            } else {
                ui.label(format!("Selected: Entity {:?}", entity.index()));
            }

            ui.separator();

            // Transform section
            if let Ok(mut transform) = transforms.get_mut(entity) {
                let changed = show_transform_editor(ui, &mut transform);
                if changed {
                    scene_state.mark_changed();
                }
            }

            ui.separator();

            // Physics section
            if let Ok(mut physics) = physics_query.get_mut(entity) {
                let changed = show_physics_editor(ui, &mut physics);
                if changed {
                    scene_state.mark_changed();
                }
            }

            ui.separator();

            // Visual section
            if let Ok(mut visual) = visual_query.get_mut(entity) {
                let changed =
                    show_visual_editor(ui, &mut visual, entity, &material_handles, &mut materials);
                if changed {
                    scene_state.mark_changed();
                }
            }

            ui.separator();

            // Copy as YAML button
            ui.horizontal(|ui| {
                if ui.button("Copy as YAML").clicked() {
                    if let Ok(transform) = transforms.get(entity) {
                        let yaml = generate_object_yaml(entity, &transform, &names, &physics_query);
                        ui.output_mut(|o| o.copied_text = yaml.clone());
                        info!("Copied object YAML to clipboard");
                    }
                }

                if ui.button("Delete").clicked() {
                    // TODO: Send delete event
                    info!("Delete requested for entity {:?}", entity);
                }
            });
        });
}

fn show_transform_editor(ui: &mut egui::Ui, transform: &mut Transform) -> bool {
    let mut changed = false;

    ui.collapsing("Transform", |ui| {
        // Position
        ui.label("Position:");
        ui.horizontal(|ui| {
            ui.label("X:");
            if ui
                .add(egui::DragValue::new(&mut transform.translation.x).speed(0.1))
                .changed()
            {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Y:");
            if ui
                .add(egui::DragValue::new(&mut transform.translation.y).speed(0.1))
                .changed()
            {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Z:");
            if ui
                .add(egui::DragValue::new(&mut transform.translation.z).speed(0.1))
                .changed()
            {
                changed = true;
            }
        });

        ui.add_space(5.0);

        // Rotation (Euler angles)
        ui.label("Rotation (degrees):");
        let (mut rx, mut ry, mut rz) = transform.rotation.to_euler(EulerRot::XYZ);
        rx = rx.to_degrees();
        ry = ry.to_degrees();
        rz = rz.to_degrees();

        ui.horizontal(|ui| {
            ui.label("X:");
            if ui.add(egui::DragValue::new(&mut rx).speed(1.0)).changed() {
                transform.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    rx.to_radians(),
                    ry.to_radians(),
                    rz.to_radians(),
                );
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Y:");
            if ui.add(egui::DragValue::new(&mut ry).speed(1.0)).changed() {
                transform.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    rx.to_radians(),
                    ry.to_radians(),
                    rz.to_radians(),
                );
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Z:");
            if ui.add(egui::DragValue::new(&mut rz).speed(1.0)).changed() {
                transform.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    rx.to_radians(),
                    ry.to_radians(),
                    rz.to_radians(),
                );
                changed = true;
            }
        });

        ui.add_space(5.0);

        // Scale
        ui.label("Scale:");
        ui.horizontal(|ui| {
            ui.label("X:");
            if ui
                .add(
                    egui::DragValue::new(&mut transform.scale.x)
                        .speed(0.01)
                        .range(0.01..=100.0),
                )
                .changed()
            {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Y:");
            if ui
                .add(
                    egui::DragValue::new(&mut transform.scale.y)
                        .speed(0.01)
                        .range(0.01..=100.0),
                )
                .changed()
            {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Z:");
            if ui
                .add(
                    egui::DragValue::new(&mut transform.scale.z)
                        .speed(0.01)
                        .range(0.01..=100.0),
                )
                .changed()
            {
                changed = true;
            }
        });

        // Reset button
        if ui.button("Reset Transform").clicked() {
            *transform = Transform::IDENTITY;
            changed = true;
        }
    })
    .header_response
    .clicked();

    changed
}

fn show_physics_editor(ui: &mut egui::Ui, physics: &mut EditablePhysics) -> bool {
    let mut changed = false;

    ui.collapsing("Physics", |ui| {
        // Static checkbox
        if ui
            .checkbox(&mut physics.is_static, "Static (immovable)")
            .changed()
        {
            changed = true;
        }

        if !physics.is_static {
            // Mass slider
            ui.horizontal(|ui| {
                ui.label("Mass (kg):");
                if ui
                    .add(
                        egui::Slider::new(&mut physics.mass, 0.1..=100.0)
                            .logarithmic(true)
                            .clamping(egui::SliderClamping::Always),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
        }

        // Friction slider
        ui.horizontal(|ui| {
            ui.label("Friction:");
            if ui
                .add(egui::Slider::new(&mut physics.friction, 0.0..=1.0))
                .changed()
            {
                changed = true;
            }
        });

        // Restitution (bounciness) slider
        ui.horizontal(|ui| {
            ui.label("Bounciness:");
            if ui
                .add(egui::Slider::new(&mut physics.restitution, 0.0..=1.0))
                .changed()
            {
                changed = true;
            }
        });

        if !physics.is_static {
            // Damping sliders
            ui.collapsing("Damping", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Linear:");
                    if ui
                        .add(egui::Slider::new(&mut physics.linear_damping, 0.0..=10.0))
                        .changed()
                    {
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Angular:");
                    if ui
                        .add(egui::Slider::new(&mut physics.angular_damping, 0.0..=10.0))
                        .changed()
                    {
                        changed = true;
                    }
                });
            });
        }

        // Presets
        ui.horizontal(|ui| {
            ui.label("Presets:");
            if ui.button("Metal").clicked() {
                physics.friction = 0.3;
                physics.restitution = 0.1;
                changed = true;
            }
            if ui.button("Rubber").clicked() {
                physics.friction = 0.9;
                physics.restitution = 0.8;
                changed = true;
            }
            if ui.button("Ice").clicked() {
                physics.friction = 0.05;
                physics.restitution = 0.1;
                changed = true;
            }
        });
    });

    changed
}

fn show_visual_editor(
    ui: &mut egui::Ui,
    visual: &mut EditableVisual,
    entity: Entity,
    material_handles: &Query<&MeshMaterial3d<StandardMaterial>>,
    materials: &mut Assets<StandardMaterial>,
) -> bool {
    let mut changed = false;

    ui.collapsing("Visual", |ui| {
        // Color picker
        ui.horizontal(|ui| {
            ui.label("Color:");
            let mut color_rgb = [visual.color[0], visual.color[1], visual.color[2]];
            if ui.color_edit_button_rgb(&mut color_rgb).changed() {
                visual.color[0] = color_rgb[0];
                visual.color[1] = color_rgb[1];
                visual.color[2] = color_rgb[2];
                changed = true;

                // Apply to material
                if let Ok(handle) = material_handles.get(entity) {
                    if let Some(material) = materials.get_mut(&handle.0) {
                        material.base_color = Color::srgba(
                            visual.color[0],
                            visual.color[1],
                            visual.color[2],
                            visual.color[3],
                        );
                    }
                }
            }
        });

        // Alpha slider
        ui.horizontal(|ui| {
            ui.label("Opacity:");
            if ui
                .add(egui::Slider::new(&mut visual.color[3], 0.0..=1.0))
                .changed()
            {
                changed = true;
                // Apply to material
                if let Ok(handle) = material_handles.get(entity) {
                    if let Some(material) = materials.get_mut(&handle.0) {
                        material.base_color = Color::srgba(
                            visual.color[0],
                            visual.color[1],
                            visual.color[2],
                            visual.color[3],
                        );
                    }
                }
            }
        });

        // Metallic slider
        ui.horizontal(|ui| {
            ui.label("Metallic:");
            if ui
                .add(egui::Slider::new(&mut visual.metallic, 0.0..=1.0))
                .changed()
            {
                changed = true;
                if let Ok(handle) = material_handles.get(entity) {
                    if let Some(material) = materials.get_mut(&handle.0) {
                        material.metallic = visual.metallic;
                    }
                }
            }
        });

        // Roughness slider
        ui.horizontal(|ui| {
            ui.label("Roughness:");
            if ui
                .add(egui::Slider::new(&mut visual.roughness, 0.0..=1.0))
                .changed()
            {
                changed = true;
                if let Ok(handle) = material_handles.get(entity) {
                    if let Some(material) = materials.get_mut(&handle.0) {
                        material.perceptual_roughness = visual.roughness;
                    }
                }
            }
        });

        // Color presets
        ui.horizontal(|ui| {
            ui.label("Presets:");
            if ui.button("Red").clicked() {
                visual.color = [0.9, 0.2, 0.2, 1.0];
                changed = true;
            }
            if ui.button("Green").clicked() {
                visual.color = [0.2, 0.8, 0.2, 1.0];
                changed = true;
            }
            if ui.button("Blue").clicked() {
                visual.color = [0.2, 0.4, 0.9, 1.0];
                changed = true;
            }
            if ui.button("Gold").clicked() {
                visual.color = [1.0, 0.84, 0.0, 1.0];
                visual.metallic = 1.0;
                visual.roughness = 0.3;
                changed = true;
            }
        });
    });

    changed
}

/// Generate YAML for a single object
fn generate_object_yaml(
    entity: Entity,
    transform: &Transform,
    names: &Query<&Name>,
    physics_query: &Query<&mut EditablePhysics>,
) -> String {
    let name = names.get(entity).map(|n| n.as_str()).unwrap_or("object");

    let (rx, ry, rz) = transform.rotation.to_euler(EulerRot::XYZ);

    let physics_str = if let Ok(physics) = physics_query.get(entity) {
        format!(
            "mass: {:.2}\n  friction: {:.2}\n  restitution: {:.2}\n  is_static: {}",
            physics.mass, physics.friction, physics.restitution, physics.is_static
        )
    } else {
        "mass: 1.0\n  friction: 0.5\n  restitution: 0.3".to_string()
    };

    format!(
        r#"- name: "{}"
  shape:
    type: box
    size: [{:.2}, {:.2}, {:.2}]
  position: [{:.2}, {:.2}, {:.2}]
  rotation_euler: [{:.1}, {:.1}, {:.1}]
  {}
"#,
        name,
        transform.scale.x,
        transform.scale.y,
        transform.scale.z,
        transform.translation.x,
        transform.translation.y,
        transform.translation.z,
        rx.to_degrees(),
        ry.to_degrees(),
        rz.to_degrees(),
        physics_str
    )
}

/// Plugin for property editor
pub struct PropertyEditorPlugin;

impl Plugin for PropertyEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, property_editor_panel_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editable_physics_default() {
        let physics = EditablePhysics::default();
        assert_eq!(physics.mass, 1.0);
        assert_eq!(physics.friction, 0.5);
        assert!(!physics.is_static);
    }

    #[test]
    fn test_editable_visual_default() {
        let visual = EditableVisual::default();
        assert_eq!(visual.color[3], 1.0); // Full opacity
        assert_eq!(visual.metallic, 0.0);
    }
}
