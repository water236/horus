//! Command-line interface for sim3d binary and validation tools

pub mod validation;

// Re-export validation types for use by horus_manager
pub use validation::{
    format_batch_report, validate_file, BatchValidationReport, OutputFormat, ValidationType,
};

use bevy::prelude::Resource;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// HORUS 3D Robotics Simulator
#[derive(Parser, Debug, Clone, Resource)]
#[command(name = "sim3d")]
#[command(about = "HORUS 3D Robotics Simulator", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(short, long, value_enum, default_value_t = Mode::Visual)]
    pub mode: Mode,

    #[arg(short, long)]
    pub robot: Option<PathBuf>,

    #[arg(short, long)]
    pub world: Option<PathBuf>,

    #[arg(short, long)]
    pub config: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    pub no_gui: bool,

    #[arg(long, default_value_t = false)]
    pub hf_viz: bool,

    #[arg(long, default_value_t = 1.0)]
    pub speed: f32,

    /// HORUS session ID (deprecated - ignored, all topics use flat namespace)
    ///
    /// **Deprecated**: Session IDs are no longer used. All topics now use a flat
    /// global namespace (ROS-like). This option is kept for backwards compatibility
    /// but has no effect.
    #[arg(long, hide = true)]
    pub session: Option<String>,

    /// Robot name for HORUS topics (default: sim3d_robot)
    #[arg(long, default_value = "sim3d_robot")]
    pub robot_name: String,

    /// Capture a screenshot and exit (internal - for automated testing via horus_mcp)
    #[arg(long, hide = true)]
    pub screenshot: Option<PathBuf>,

    /// Frames to wait before capturing screenshot (default: 30)
    #[arg(long, hide = true, default_value_t = 30)]
    pub screenshot_frames: u32,

    /// Camera angle for screenshot (internal - for multi-angle verification)
    /// Options: front, back, left, right, top, isometric (default: isometric)
    #[arg(long, hide = true, default_value = "isometric")]
    pub screenshot_angle: String,

    /// Camera distance multiplier for screenshot (internal - default: 1.0)
    #[arg(long, hide = true, default_value_t = 1.0)]
    pub screenshot_distance: f32,

    /// Focus on world overview instead of robot close-up (internal)
    #[arg(long, hide = true, default_value_t = false)]
    pub screenshot_world: bool,
}

/// Subcommands for sim3d
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Validate scene or URDF files
    Validate {
        /// Files to validate
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Validation type (auto-detect if not specified)
        #[arg(short = 't', long, value_enum)]
        validation_type: Option<CliValidationType>,

        /// Output format
        #[arg(short, long, value_enum, default_value_t = CliOutputFormat::Text)]
        format: CliOutputFormat,

        /// Check mesh references exist
        #[arg(long, default_value_t = true)]
        check_meshes: bool,

        /// Verbose output
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliValidationType {
    Scene,
    Urdf,
    Auto,
}

impl From<CliValidationType> for ValidationType {
    fn from(val: CliValidationType) -> Self {
        match val {
            CliValidationType::Scene => ValidationType::Scene,
            CliValidationType::Urdf => ValidationType::Urdf,
            CliValidationType::Auto => ValidationType::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliOutputFormat {
    Text,
    Json,
    Html,
}

impl From<CliOutputFormat> for OutputFormat {
    fn from(val: CliOutputFormat) -> Self {
        match val {
            CliOutputFormat::Text => OutputFormat::Text,
            CliOutputFormat::Json => OutputFormat::Json,
            CliOutputFormat::Html => OutputFormat::Html,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Resource)]
pub enum Mode {
    Visual,
    Headless,
}

impl Cli {
    pub fn parse() -> Self {
        Parser::parse()
    }

    /// Check if a subcommand was provided (not the default run mode)
    pub fn has_subcommand(&self) -> bool {
        self.command.is_some()
    }
}
