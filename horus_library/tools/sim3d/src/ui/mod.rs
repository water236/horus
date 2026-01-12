pub mod controls;
pub mod crash_recovery;
pub mod debug_panel;
#[cfg(feature = "visual")]
pub mod dock;
pub mod file_dialog;
#[cfg(feature = "visual")]
pub mod floating_panels;
pub mod hframe_panel;
pub mod horus_panel;
pub mod keybindings;
pub mod layouts;
pub mod notifications;
#[cfg(feature = "visual")]
pub mod panel_manager;
#[cfg(feature = "visual")]
pub mod panel_state;
pub mod physics_panel;
pub mod plugin_panel;
pub mod recent_files;
pub mod recording_panel;
pub mod rendering_panel;
pub mod sensor_panel;
pub mod stats_panel;
pub mod status_bar;
pub mod theme;
pub mod tooltips;
pub mod view_modes;

// Re-export theme components for convenience

// Re-export notification components for convenience

// Re-export status bar components for convenience

// Re-export keybindings for convenience

// Re-export layouts components for convenience

// Re-export tooltips components for convenience

// Re-export recent files components for convenience

// Re-export crash recovery components for convenience

// Re-export plugin panel components for convenience

// Re-export dock components for convenience
