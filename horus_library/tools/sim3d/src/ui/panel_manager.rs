//! Panel Manager
//!
//! Central resource for managing both docked and floating panels.
//! Handles panel actions (detach, dock, toggle) and coordinates between
//! the egui_dock system and floating egui::Windows.

use bevy::prelude::*;
use egui_dock::DockState;
use std::collections::VecDeque;

use super::dock::{DockTab, DockWorkspace};
use super::keybindings::{KeyBindingAction, KeyBindingTriggeredEvent};
use super::panel_state::{FloatingPanelConfig, FloatingPanelState, PanelAction};

/// Central manager for all panel states and actions
#[derive(Resource)]
pub struct PanelManager {
    /// Tracks which panels are floating and their configurations
    pub floating: FloatingPanelState,
    /// Queue of pending actions to process at end of frame
    pending_actions: VecDeque<PanelAction>,
    /// Hidden panels (not in dock and not floating)
    hidden: Vec<DockTab>,
    /// Whether the panel manager is enabled
    pub enabled: bool,
}

impl Default for PanelManager {
    fn default() -> Self {
        Self {
            floating: FloatingPanelState::new(),
            pending_actions: VecDeque::new(),
            hidden: Vec::new(),
            enabled: true,
        }
    }
}

impl PanelManager {
    /// Create a new panel manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue an action to be processed at end of frame
    pub fn queue_action(&mut self, action: PanelAction) {
        self.pending_actions.push_back(action);
    }

    /// Queue multiple actions
    pub fn queue_actions(&mut self, actions: impl IntoIterator<Item = PanelAction>) {
        self.pending_actions.extend(actions);
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Get number of pending actions
    pub fn pending_action_count(&self) -> usize {
        self.pending_actions.len()
    }

    /// Check if a tab is currently floating
    pub fn is_floating(&self, tab: &DockTab) -> bool {
        self.floating.is_floating(tab)
    }

    /// Check if a tab is currently hidden
    pub fn is_hidden(&self, tab: &DockTab) -> bool {
        self.hidden.contains(tab)
    }

    /// Get all hidden tabs
    pub fn hidden_tabs(&self) -> &[DockTab] {
        &self.hidden
    }

    /// Process all pending actions
    /// Returns the actions that were processed (for logging/debugging)
    pub fn process_actions(&mut self, workspace: &mut DockWorkspace) -> Vec<PanelAction> {
        let mut processed = Vec::with_capacity(self.pending_actions.len());

        while let Some(action) = self.pending_actions.pop_front() {
            self.execute_action(&action, workspace);
            processed.push(action);
        }

        processed
    }

    /// Execute a single action immediately
    fn execute_action(&mut self, action: &PanelAction, workspace: &mut DockWorkspace) {
        match action {
            PanelAction::Detach(tab) => {
                self.detach_tab(tab.clone(), workspace);
            }
            PanelAction::Dock(tab) => {
                self.dock_tab(tab.clone(), workspace);
            }
            PanelAction::BringToFront(_tab) => {
                // egui handles z-order via render order, which we handle
                // by iterating floating tabs in a specific order
            }
            PanelAction::ToggleCollapse(tab) => {
                self.floating.update_config(tab, |config| {
                    config.collapsed = !config.collapsed;
                });
            }
            PanelAction::ToggleAlwaysOnTop(tab) => {
                self.floating.update_config(tab, |config| {
                    config.always_on_top = !config.always_on_top;
                });
            }
            PanelAction::Close(tab) => {
                self.close_tab(tab.clone(), workspace);
            }
            PanelAction::Show(tab) => {
                self.show_tab(tab.clone(), workspace);
            }
            PanelAction::Toggle(tab) => {
                if self.is_hidden(tab) {
                    self.show_tab(tab.clone(), workspace);
                } else {
                    self.close_tab(tab.clone(), workspace);
                }
            }
        }
    }

    /// Detach a tab from dock to floating window
    fn detach_tab(&mut self, tab: DockTab, workspace: &mut DockWorkspace) {
        // Remove from dock if present
        if let Some(_removed) = remove_tab_from_dock(&mut workspace.state, &tab) {
            // Add to floating with default config
            let config = FloatingPanelConfig::default();
            self.floating.detach(&tab, config);
            tracing::debug!("Detached tab {:?} to floating window", tab);
        }
    }

    /// Dock a floating tab back into the dock system
    fn dock_tab(&mut self, tab: DockTab, workspace: &mut DockWorkspace) {
        // Remove from floating
        if let Some(_config) = self.floating.dock(&tab) {
            // Add back to dock
            add_tab_to_dock(&mut workspace.state, tab.clone());
            tracing::debug!("Docked floating tab {:?}", tab);
        }

        // Also remove from hidden if it was there
        self.hidden.retain(|t| t != &tab);
    }

    /// Close a tab entirely (remove from dock and floating)
    fn close_tab(&mut self, tab: DockTab, workspace: &mut DockWorkspace) {
        // Remove from dock
        let _ = remove_tab_from_dock(&mut workspace.state, &tab);

        // Remove from floating
        self.floating.dock(&tab);

        // Add to hidden
        if !self.hidden.contains(&tab) {
            self.hidden.push(tab.clone());
        }

        tracing::debug!("Closed tab {:?}", tab);
    }

    /// Show a hidden tab (add to dock)
    fn show_tab(&mut self, tab: DockTab, workspace: &mut DockWorkspace) {
        // Remove from hidden
        self.hidden.retain(|t| t != &tab);

        // Add to dock (not floating)
        add_tab_to_dock(&mut workspace.state, tab.clone());

        tracing::debug!("Showed tab {:?}", tab);
    }

    /// Get floating panel configuration for a tab
    pub fn get_floating_config(&self, tab: &DockTab) -> Option<&FloatingPanelConfig> {
        self.floating.get_config(tab)
    }

    /// Update floating panel configuration
    pub fn update_floating_config<F>(&mut self, tab: &DockTab, f: F)
    where
        F: FnOnce(&mut FloatingPanelConfig),
    {
        self.floating.update_config(tab, f);
    }

    /// Dock all floating panels
    pub fn dock_all(&mut self, workspace: &mut DockWorkspace) {
        let floating_tabs: Vec<DockTab> = self.floating.floating_tabs().collect();
        for tab in floating_tabs {
            self.dock_tab(tab, workspace);
        }
    }

    /// Detach all docked panels to floating
    pub fn detach_all(&mut self, workspace: &mut DockWorkspace) {
        // Get all tabs from dock
        let docked_tabs = get_all_tabs_from_dock(&workspace.state);
        for tab in docked_tabs {
            self.detach_tab(tab, workspace);
        }
    }
}

/// Remove a tab from the dock state
/// Returns the removed tab if found
fn remove_tab_from_dock(dock_state: &mut DockState<DockTab>, tab: &DockTab) -> Option<DockTab> {
    // Find the tab location using egui_dock's find_tab method
    if let Some((surface_idx, node_idx, tab_idx)) = dock_state.find_tab(tab) {
        // Use DockState's remove_tab method which handles surfaces correctly
        return dock_state.remove_tab((surface_idx, node_idx, tab_idx));
    }
    None
}

/// Add a tab to the dock state (to the focused leaf or root)
fn add_tab_to_dock(dock_state: &mut DockState<DockTab>, tab: DockTab) {
    dock_state.main_surface_mut().push_to_focused_leaf(tab);
}

/// Get all tabs currently in the dock state
fn get_all_tabs_from_dock(dock_state: &DockState<DockTab>) -> Vec<DockTab> {
    // Use iter_all_tabs which properly iterates all tabs across all surfaces
    dock_state
        .iter_all_tabs()
        .map(|(_, tab)| tab.clone())
        .collect()
}

/// Event for panel actions (allows other systems to trigger panel operations)
#[derive(Event, Debug, Clone)]
pub struct PanelActionEvent(pub PanelAction);

/// System to collect panel action events and queue them
pub fn collect_panel_action_events(
    mut events: EventReader<PanelActionEvent>,
    mut panel_manager: ResMut<PanelManager>,
) {
    for event in events.read() {
        panel_manager.queue_action(event.0.clone());
    }
}

/// System to process queued panel actions at end of frame
pub fn process_panel_actions(
    mut panel_manager: ResMut<PanelManager>,
    mut workspace: ResMut<DockWorkspace>,
) {
    if panel_manager.has_pending_actions() {
        let processed = panel_manager.process_actions(&mut workspace);
        if !processed.is_empty() {
            tracing::trace!("Processed {} panel actions", processed.len());
        }
    }
}

/// System to handle keybinding events and convert them to panel actions
pub fn handle_keybinding_panel_actions(
    mut keybinding_events: EventReader<KeyBindingTriggeredEvent>,
    mut panel_manager: ResMut<PanelManager>,
    mut workspace: ResMut<DockWorkspace>,
) {
    for event in keybinding_events.read() {
        match event.action {
            KeyBindingAction::ToggleControls => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::Controls));
            }
            KeyBindingAction::TogglePhysicsPanel => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::Physics));
            }
            KeyBindingAction::ToggleSensors => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::Sensors));
            }
            KeyBindingAction::ToggleRendering => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::Rendering));
            }
            KeyBindingAction::ToggleRecording => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::Recording));
            }
            KeyBindingAction::ToggleHorusPanel => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::Horus));
            }
            KeyBindingAction::ToggleHFramePanel => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::HFrameTree));
            }
            KeyBindingAction::ToggleViewModes => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::ViewModes));
            }
            KeyBindingAction::ToggleStats => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::Stats));
            }
            KeyBindingAction::ToggleConsole => {
                panel_manager.queue_action(PanelAction::Toggle(DockTab::Console));
            }
            KeyBindingAction::DockAllPanels => {
                panel_manager.dock_all(&mut workspace);
            }
            KeyBindingAction::DetachAllPanels => {
                panel_manager.detach_all(&mut workspace);
            }
            _ => {}
        }
    }
}

/// Plugin that adds the PanelManager system
pub struct PanelManagerPlugin;

impl Plugin for PanelManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelManager>()
            .add_event::<PanelActionEvent>()
            .add_systems(
                Update,
                (
                    handle_keybinding_panel_actions,
                    collect_panel_action_events,
                    process_panel_actions,
                )
                    .chain(),
            );

        tracing::info!("PanelManager initialized - hybrid dock/floating system enabled");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_manager_default() {
        let pm = PanelManager::default();
        assert!(pm.enabled);
        assert!(!pm.has_pending_actions());
        assert_eq!(pm.floating.floating_count(), 0);
    }

    #[test]
    fn test_queue_actions() {
        let mut pm = PanelManager::new();

        pm.queue_action(PanelAction::Detach(DockTab::Stats));
        pm.queue_action(PanelAction::Dock(DockTab::Console));

        assert!(pm.has_pending_actions());
        assert_eq!(pm.pending_action_count(), 2);
    }

    #[test]
    fn test_hidden_tabs() {
        let mut pm = PanelManager::new();
        assert!(!pm.is_hidden(&DockTab::Stats));

        pm.hidden.push(DockTab::Stats);
        assert!(pm.is_hidden(&DockTab::Stats));
    }
}
