//! Floating Panel Rendering System
//!
//! Renders detached panels as floating egui::Windows, complementing the
//! egui_dock docked panel system. Provides a hybrid UI where panels can
//! be either docked or floating based on user preference.

use bevy::prelude::*;
use bevy_egui::egui;

use super::dock::{DockRenderContext, DockTab, DockWorkspace};
use super::panel_manager::PanelManager;
use super::panel_state::PanelAction;

use crate::systems::horus_sync::HorusSyncStats;
use crate::ui::controls::render_controls_ui;
use crate::ui::hframe_panel::render_hframe_ui;
use crate::ui::horus_panel::render_horus_panel_ui;
use crate::ui::physics_panel::render_physics_panel_ui;
use crate::ui::recording_panel::render_recording_panel_ui;
use crate::ui::rendering_panel::render_rendering_panel_ui;
use crate::ui::sensor_panel::render_sensor_panel_ui;
use crate::ui::stats_panel::render_stats_ui;
use crate::ui::view_modes::render_view_modes_ui;

/// Events collected during floating panel rendering
pub struct FloatingPanelEvents {
    /// Panel actions to queue (dock, close, etc.)
    pub panel_actions: Vec<PanelAction>,
    /// Simulation events from Controls panel
    pub simulation_events: Vec<crate::ui::controls::SimulationEvent>,
    /// Recording events from Recording panel
    pub recording_events: Vec<crate::ui::recording_panel::RecordingEvent>,
}

impl Default for FloatingPanelEvents {
    fn default() -> Self {
        Self {
            panel_actions: Vec::new(),
            simulation_events: Vec::new(),
            recording_events: Vec::new(),
        }
    }
}

/// Render all floating panels
///
/// This function iterates over panels marked as floating in the PanelManager
/// and renders each as an egui::Window with:
/// - Title bar with dock button
/// - Panel-specific content
/// - Close handling (auto-docks on close)
///
/// Returns collected events that the caller should dispatch.
pub fn render_floating_panels(
    egui_ctx: &egui::Context,
    panel_manager: &mut PanelManager,
    ctx: &mut DockRenderContext,
    console_messages: &mut Vec<String>,
) -> FloatingPanelEvents {
    let mut events = FloatingPanelEvents::default();

    if !panel_manager.enabled {
        return events;
    }

    // Collect floating tabs to avoid borrow issues
    let floating_tabs: Vec<DockTab> = panel_manager.floating.floating_tabs().collect();

    for tab in floating_tabs {
        let config = match panel_manager.get_floating_config(&tab) {
            Some(cfg) => cfg.clone(),
            None => continue,
        };

        // Window ID based on tab type
        let window_id = format!("floating_{:?}", tab);

        // Track if window is open (for close detection)
        let mut is_open = true;

        // Build the window
        let mut window = egui::Window::new(tab.title())
            .id(egui::Id::new(&window_id))
            .open(&mut is_open)
            .resizable(config.resizable)
            .collapsible(true);

        // Apply position if set
        if config.position != [0.0, 0.0] {
            window = window.default_pos(egui::pos2(config.position[0], config.position[1]));
        } else {
            // Default position based on tab type to avoid stacking
            let offset = match &tab {
                DockTab::Controls => [50.0, 50.0],
                DockTab::Stats => [100.0, 100.0],
                DockTab::Console => [150.0, 150.0],
                DockTab::HFrameTree => [200.0, 200.0],
                DockTab::ViewModes => [250.0, 250.0],
                DockTab::Physics => [300.0, 300.0],
                DockTab::Sensors => [350.0, 350.0],
                DockTab::Rendering => [400.0, 400.0],
                DockTab::Recording => [450.0, 450.0],
                DockTab::Horus => [500.0, 500.0],
                DockTab::Plugin(_) => [550.0, 550.0],
            };
            window = window.default_pos(egui::pos2(offset[0], offset[1]));
        }

        // Apply size if set
        if config.size != [0.0, 0.0] {
            window = window.default_size(egui::vec2(config.size[0], config.size[1]));
        } else {
            // Default sizes based on content
            let size = match &tab {
                DockTab::Controls => [280.0, 200.0],
                DockTab::Stats => [300.0, 350.0],
                DockTab::Console => [400.0, 250.0],
                DockTab::HFrameTree => [350.0, 400.0],
                DockTab::ViewModes => [250.0, 200.0],
                DockTab::Physics => [300.0, 350.0],
                DockTab::Sensors => [320.0, 400.0],
                DockTab::Rendering => [300.0, 350.0],
                DockTab::Recording => [320.0, 380.0],
                DockTab::Horus => [350.0, 400.0],
                DockTab::Plugin(_) => [300.0, 300.0],
            };
            window = window.default_size(egui::vec2(size[0], size[1]));
        }

        // Apply opacity if not default
        if config.opacity < 1.0 {
            // egui doesn't have direct opacity control, but we can use frame
            window = window.frame(
                egui::Frame::window(&egui_ctx.style()).multiply_with_opacity(config.opacity),
            );
        }

        window.show(egui_ctx, |ui| {
            // Title bar with dock button
            ui.horizontal(|ui| {
                if ui
                    .button("[Dock]")
                    .on_hover_text("Dock this panel back into the main dock area")
                    .clicked()
                {
                    events.panel_actions.push(PanelAction::Dock(tab.clone()));
                }

                ui.separator();

                // Additional window controls
                if ui.button("[^]").on_hover_text("Always on top").clicked() {
                    events
                        .panel_actions
                        .push(PanelAction::ToggleAlwaysOnTop(tab.clone()));
                }
            });

            ui.separator();

            // Render panel content based on tab type
            render_floating_panel_content(ui, &tab, ctx, console_messages, &mut events);
        });

        // If window was closed, queue a dock action
        if !is_open {
            events.panel_actions.push(PanelAction::Dock(tab.clone()));
        }
    }

    events
}

/// Render the content for a specific floating panel tab
fn render_floating_panel_content(
    ui: &mut egui::Ui,
    tab: &DockTab,
    ctx: &mut DockRenderContext,
    console_messages: &mut Vec<String>,
    events: &mut FloatingPanelEvents,
) {
    match tab {
        DockTab::Controls => {
            let control_events = render_controls_ui(ui, ctx.controls);
            events.simulation_events.extend(control_events.events);
        }
        DockTab::Stats => {
            render_stats_ui(ui, ctx.time, ctx.stats, ctx.frame_time, ctx.horus_stats);
        }
        DockTab::Console => {
            render_console_content(ui, console_messages);
        }
        DockTab::HFrameTree => {
            render_hframe_ui(
                ui,
                ctx.hframe_tree,
                ctx.hframe_panel_config,
                &ctx.hframe_publishers,
                ctx.current_time,
            );
        }
        DockTab::ViewModes => {
            render_view_modes_ui(ui, ctx.view_mode);
        }
        DockTab::Physics => {
            render_physics_panel_ui(
                ui,
                ctx.physics_params,
                ctx.physics_panel_config,
                ctx.physics_world,
            );
        }
        DockTab::Sensors => {
            render_sensor_panel_ui(ui, ctx.sensor_settings, ctx.sensor_panel_config);
        }
        DockTab::Rendering => {
            render_rendering_panel_ui(ui, ctx.rendering_settings, ctx.rendering_panel_config);
        }
        DockTab::Recording => {
            let rec_events =
                render_recording_panel_ui(ui, ctx.recording_settings, ctx.recording_panel_config);
            events.recording_events.extend(rec_events);
        }
        DockTab::Horus => {
            if let Some(horus_comm) = ctx.horus_comm {
                render_horus_panel_ui(
                    ui,
                    ctx.horus_panel_config,
                    horus_comm,
                    ctx.horus_sync_config,
                    ctx.horus_stats.unwrap_or(&HorusSyncStats::default()),
                    ctx.topic_scanner,
                    ctx.topic_subscriptions.as_deref_mut(),
                );
            } else {
                ui.label("HORUS not initialized");
            }
        }
        DockTab::Plugin(name) => {
            ui.heading(format!("Plugin: {}", name));
            ui.separator();
            ui.label("Plugin-specific content goes here");
        }
    }
}

/// Render console content (mirrors dock.rs implementation)
fn render_console_content(ui: &mut egui::Ui, messages: &mut Vec<String>) {
    ui.heading("Console");
    ui.separator();

    // Add clear button
    ui.horizontal(|ui| {
        if ui.button("Clear").clicked() {
            messages.clear();
        }
        ui.label(format!("{} messages", messages.len()));
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for msg in messages.iter() {
                // Color code by log level
                if msg.contains("[ERROR]") {
                    ui.colored_label(egui::Color32::RED, msg);
                } else if msg.contains("[WARN]") {
                    ui.colored_label(egui::Color32::YELLOW, msg);
                } else if msg.contains("[INFO]") {
                    ui.label(msg);
                } else {
                    ui.colored_label(egui::Color32::GRAY, msg);
                }
            }
        });
}

/// System to render floating panels
///
/// This system runs after the dock system and renders any panels
/// that have been detached to floating windows.
#[cfg(feature = "visual")]
pub fn floating_panels_system(
    mut contexts: bevy_egui::EguiContexts,
    mut panel_manager: ResMut<PanelManager>,
    mut workspace: ResMut<DockWorkspace>,
    mut console: ResMut<super::dock::ConsoleMessages>,
    mut sim_events: EventWriter<crate::ui::controls::SimulationEvent>,
    // Bundled resources
    mut core_ui: super::dock::CoreUiResources,
    mut panel_resources: super::dock::PanelResources,
) {
    if !panel_manager.enabled {
        return;
    }

    // Skip if no floating panels
    if panel_manager.floating.floating_count() == 0 {
        return;
    }

    let egui_ctx = contexts.ctx_mut();

    // Collect HFrame publishers
    let hframe_pubs: Vec<&crate::systems::hframe_update::HFramePublisher> =
        core_ui.hframe_publishers.iter().collect();

    // Build render context
    let mut render_ctx = DockRenderContext {
        time: &core_ui.time,
        stats: &core_ui.stats,
        frame_time: &core_ui.frame_time,
        controls: &mut core_ui.controls,
        hframe_tree: &core_ui.hframe_tree,
        physics_world: core_ui.physics_world.as_deref(),
        horus_stats: core_ui.horus_stats.as_deref(),
        hframe_publishers: hframe_pubs,
        hframe_panel_config: &mut core_ui.hframe_panel_config,
        view_mode: &mut core_ui.view_mode,
        current_time: core_ui.time.elapsed_secs(),
        // Panel resources
        physics_params: &mut panel_resources.physics_params,
        physics_panel_config: &mut panel_resources.physics_panel_config,
        sensor_settings: &mut panel_resources.sensor_settings,
        sensor_panel_config: &mut panel_resources.sensor_panel_config,
        rendering_settings: &mut panel_resources.rendering_settings,
        rendering_panel_config: &mut panel_resources.rendering_panel_config,
        recording_settings: &mut panel_resources.recording_settings,
        recording_panel_config: &mut panel_resources.recording_panel_config,
        // HORUS panel resources
        horus_comm: panel_resources.horus_comm.as_deref(),
        horus_sync_config: &mut panel_resources.horus_sync_config,
        horus_panel_config: &mut panel_resources.horus_panel_config,
        topic_scanner: panel_resources.topic_scanner.as_deref(),
        topic_subscriptions: panel_resources.topic_subscriptions.as_deref_mut(),
    };

    // Render floating panels
    let events = render_floating_panels(
        egui_ctx,
        &mut panel_manager,
        &mut render_ctx,
        &mut console.messages,
    );

    // Queue panel actions
    for action in events.panel_actions {
        panel_manager.queue_action(action);
    }

    // Process queued actions immediately
    if panel_manager.has_pending_actions() {
        let _processed = panel_manager.process_actions(&mut workspace);
    }

    // Send simulation events
    for event in events.simulation_events {
        sim_events.send(event);
    }

    // Send recording events
    for event in events.recording_events {
        panel_resources.recording_events.send(event);
    }
}

#[cfg(not(feature = "visual"))]
pub fn floating_panels_system() {}

/// Plugin that adds the floating panel system
pub struct FloatingPanelsPlugin;

impl Plugin for FloatingPanelsPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "visual")]
        {
            use bevy_egui::EguiSet;
            // Run after dock system so floating panels render on top
            app.add_systems(
                Update,
                floating_panels_system
                    .after(EguiSet::InitContexts)
                    .after(super::dock::dock_ui_system),
            );
        }

        #[cfg(not(feature = "visual"))]
        {
            app.add_systems(Update, floating_panels_system);
        }

        tracing::info!("Floating panels system initialized");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floating_panel_events_default() {
        let events = FloatingPanelEvents::default();
        assert!(events.panel_actions.is_empty());
        assert!(events.simulation_events.is_empty());
        assert!(events.recording_events.is_empty());
    }
}
