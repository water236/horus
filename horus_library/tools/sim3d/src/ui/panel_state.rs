//! Panel State Management
//!
//! Core data structures for tracking whether panels are docked within egui_dock
//! or floating as independent egui::Window instances. This enables a hybrid UI
//! approach where users can detach panels to floating windows and dock them back.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::dock::DockTab;

/// Represents whether a panel is docked within the dock system or floating as a window
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PanelState {
    /// Panel is docked within egui_dock's DockState
    Docked,
    /// Panel is floating as a separate egui::Window
    Floating(FloatingPanelConfig),
}

impl Default for PanelState {
    fn default() -> Self {
        PanelState::Docked
    }
}

impl PanelState {
    /// Check if panel is currently docked
    pub fn is_docked(&self) -> bool {
        matches!(self, PanelState::Docked)
    }

    /// Check if panel is currently floating
    pub fn is_floating(&self) -> bool {
        matches!(self, PanelState::Floating(_))
    }

    /// Get floating config if panel is floating
    pub fn floating_config(&self) -> Option<&FloatingPanelConfig> {
        match self {
            PanelState::Floating(config) => Some(config),
            PanelState::Docked => None,
        }
    }

    /// Get mutable floating config if panel is floating
    pub fn floating_config_mut(&mut self) -> Option<&mut FloatingPanelConfig> {
        match self {
            PanelState::Floating(config) => Some(config),
            PanelState::Docked => None,
        }
    }
}

/// Configuration for a floating panel window
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloatingPanelConfig {
    /// Screen position (x, y) in logical pixels
    pub position: [f32; 2],
    /// Window size (width, height) in logical pixels
    pub size: [f32; 2],
    /// Whether the window is collapsed (minimized to title bar)
    pub collapsed: bool,
    /// Whether the window stays on top of other windows
    pub always_on_top: bool,
    /// Window opacity (0.0 = transparent, 1.0 = opaque)
    pub opacity: f32,
    /// Whether the window is resizable
    pub resizable: bool,
    /// Whether the window has a title bar
    pub title_bar: bool,
}

impl Default for FloatingPanelConfig {
    fn default() -> Self {
        Self {
            position: [100.0, 100.0],
            size: [300.0, 400.0],
            collapsed: false,
            always_on_top: false,
            opacity: 1.0,
            resizable: true,
            title_bar: true,
        }
    }
}

impl FloatingPanelConfig {
    /// Create a new floating panel config with specified position
    pub fn at_position(x: f32, y: f32) -> Self {
        Self {
            position: [x, y],
            ..Default::default()
        }
    }

    /// Create a new floating panel config with specified size
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = [width, height];
        self
    }

    /// Set always on top
    pub fn always_on_top(mut self, on_top: bool) -> Self {
        self.always_on_top = on_top;
        self
    }

    /// Set opacity
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set collapsed state
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Get position as egui Pos2
    #[cfg(feature = "visual")]
    pub fn egui_position(&self) -> bevy_egui::egui::Pos2 {
        bevy_egui::egui::pos2(self.position[0], self.position[1])
    }

    /// Get size as egui Vec2
    #[cfg(feature = "visual")]
    pub fn egui_size(&self) -> bevy_egui::egui::Vec2 {
        bevy_egui::egui::vec2(self.size[0], self.size[1])
    }

    /// Update position from egui Pos2
    #[cfg(feature = "visual")]
    pub fn set_egui_position(&mut self, pos: bevy_egui::egui::Pos2) {
        self.position = [pos.x, pos.y];
    }

    /// Update size from egui Vec2
    #[cfg(feature = "visual")]
    pub fn set_egui_size(&mut self, size: bevy_egui::egui::Vec2) {
        self.size = [size.x, size.y];
    }
}

/// Actions that can be performed on panels
#[derive(Debug, Clone, PartialEq)]
pub enum PanelAction {
    /// Detach a panel from dock to floating window
    Detach(DockTab),
    /// Dock a floating panel back into the dock system
    Dock(DockTab),
    /// Bring a floating panel to front (update z-order)
    BringToFront(DockTab),
    /// Toggle collapsed state of a floating panel
    ToggleCollapse(DockTab),
    /// Toggle always-on-top state of a floating panel
    ToggleAlwaysOnTop(DockTab),
    /// Close a panel entirely (remove from dock and floating)
    Close(DockTab),
    /// Show a hidden panel (add to dock)
    Show(DockTab),
    /// Toggle panel visibility
    Toggle(DockTab),
}

/// Tracks which panels are currently floating (detached from dock)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FloatingPanelState {
    /// Set of panels that are currently floating
    detached: HashSet<DockTabKey>,
    /// Configuration for each floating panel
    configs: HashMap<DockTabKey, FloatingPanelConfig>,
}

/// Key type for DockTab that can be serialized
/// (DockTab::Plugin contains String which needs special handling)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DockTabKey {
    Controls,
    Stats,
    Console,
    HFrameTree,
    ViewModes,
    Physics,
    Sensors,
    Rendering,
    Recording,
    Horus,
    Plugin(String),
}

impl From<&DockTab> for DockTabKey {
    fn from(tab: &DockTab) -> Self {
        match tab {
            DockTab::Controls => DockTabKey::Controls,
            DockTab::Stats => DockTabKey::Stats,
            DockTab::Console => DockTabKey::Console,
            DockTab::HFrameTree => DockTabKey::HFrameTree,
            DockTab::ViewModes => DockTabKey::ViewModes,
            DockTab::Physics => DockTabKey::Physics,
            DockTab::Sensors => DockTabKey::Sensors,
            DockTab::Rendering => DockTabKey::Rendering,
            DockTab::Recording => DockTabKey::Recording,
            DockTab::Horus => DockTabKey::Horus,
            DockTab::Plugin(name) => DockTabKey::Plugin(name.clone()),
        }
    }
}

impl From<DockTabKey> for DockTab {
    fn from(key: DockTabKey) -> Self {
        match key {
            DockTabKey::Controls => DockTab::Controls,
            DockTabKey::Stats => DockTab::Stats,
            DockTabKey::Console => DockTab::Console,
            DockTabKey::HFrameTree => DockTab::HFrameTree,
            DockTabKey::ViewModes => DockTab::ViewModes,
            DockTabKey::Physics => DockTab::Physics,
            DockTabKey::Sensors => DockTab::Sensors,
            DockTabKey::Rendering => DockTab::Rendering,
            DockTabKey::Recording => DockTab::Recording,
            DockTabKey::Horus => DockTab::Horus,
            DockTabKey::Plugin(name) => DockTab::Plugin(name),
        }
    }
}

impl DockTabKey {
    /// Convert to DockTab
    pub fn to_dock_tab(&self) -> Option<DockTab> {
        Some(DockTab::from(self.clone()))
    }
}

impl FloatingPanelState {
    /// Create a new empty floating panel state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a tab is currently floating
    pub fn is_floating(&self, tab: &DockTab) -> bool {
        self.detached.contains(&DockTabKey::from(tab))
    }

    /// Get the configuration for a floating tab
    pub fn get_config(&self, tab: &DockTab) -> Option<&FloatingPanelConfig> {
        self.configs.get(&DockTabKey::from(tab))
    }

    /// Get mutable configuration for a floating tab
    pub fn get_config_mut(&mut self, tab: &DockTab) -> Option<&mut FloatingPanelConfig> {
        self.configs.get_mut(&DockTabKey::from(tab))
    }

    /// Detach a tab to floating, returning true if it was newly detached
    pub fn detach(&mut self, tab: &DockTab, config: FloatingPanelConfig) -> bool {
        let key = DockTabKey::from(tab);
        self.configs.insert(key.clone(), config);
        self.detached.insert(key)
    }

    /// Dock a floating tab back, returning the config if it was floating
    pub fn dock(&mut self, tab: &DockTab) -> Option<FloatingPanelConfig> {
        let key = DockTabKey::from(tab);
        self.detached.remove(&key);
        self.configs.remove(&key)
    }

    /// Get all currently floating tabs
    pub fn floating_tabs(&self) -> impl Iterator<Item = DockTab> + '_ {
        self.detached.iter().map(|key| DockTab::from(key.clone()))
    }

    /// Get all currently floating tab keys (for serialization)
    pub fn floating_tab_keys(&self) -> impl Iterator<Item = &DockTabKey> {
        self.detached.iter()
    }

    /// Get number of floating panels
    pub fn floating_count(&self) -> usize {
        self.detached.len()
    }

    /// Update config for a floating panel (no-op if not floating)
    pub fn update_config<F>(&mut self, tab: &DockTab, f: F)
    where
        F: FnOnce(&mut FloatingPanelConfig),
    {
        if let Some(config) = self.get_config_mut(tab) {
            f(config);
        }
    }

    /// Clear all floating panels (dock everything)
    pub fn dock_all(&mut self) {
        self.detached.clear();
        self.configs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_state_default() {
        let state = PanelState::default();
        assert!(state.is_docked());
        assert!(!state.is_floating());
    }

    #[test]
    fn test_floating_panel_config_default() {
        let config = FloatingPanelConfig::default();
        assert_eq!(config.position, [100.0, 100.0]);
        assert_eq!(config.size, [300.0, 400.0]);
        assert!(!config.collapsed);
        assert!(!config.always_on_top);
        assert_eq!(config.opacity, 1.0);
    }

    #[test]
    fn test_floating_panel_config_builder() {
        let config = FloatingPanelConfig::at_position(200.0, 150.0)
            .with_size(400.0, 300.0)
            .with_opacity(0.9)
            .always_on_top(true);

        assert_eq!(config.position, [200.0, 150.0]);
        assert_eq!(config.size, [400.0, 300.0]);
        assert_eq!(config.opacity, 0.9);
        assert!(config.always_on_top);
    }

    #[test]
    fn test_floating_panel_state() {
        let mut state = FloatingPanelState::new();

        // Initially not floating
        assert!(!state.is_floating(&DockTab::Controls));
        assert_eq!(state.floating_count(), 0);

        // Detach
        let config = FloatingPanelConfig::at_position(50.0, 50.0);
        assert!(state.detach(&DockTab::Controls, config));
        assert!(state.is_floating(&DockTab::Controls));
        assert_eq!(state.floating_count(), 1);

        // Detach same tab again (should return false)
        let config2 = FloatingPanelConfig::default();
        assert!(!state.detach(&DockTab::Controls, config2));

        // Dock
        let returned_config = state.dock(&DockTab::Controls);
        assert!(returned_config.is_some());
        assert_eq!(returned_config.unwrap().position, [50.0, 50.0]);
        assert!(!state.is_floating(&DockTab::Controls));
        assert_eq!(state.floating_count(), 0);
    }

    #[test]
    fn test_dock_tab_key_conversion() {
        let tab = DockTab::Physics;
        let key = DockTabKey::from(&tab);
        let tab_back = DockTab::from(key);
        assert_eq!(tab, tab_back);

        let plugin_tab = DockTab::Plugin("TestPlugin".to_string());
        let plugin_key = DockTabKey::from(&plugin_tab);
        let plugin_back = DockTab::from(plugin_key);
        assert_eq!(plugin_tab, plugin_back);
    }

    #[test]
    fn test_panel_action_variants() {
        let detach = PanelAction::Detach(DockTab::Stats);
        let dock = PanelAction::Dock(DockTab::Stats);
        assert_ne!(detach, dock);
    }
}
