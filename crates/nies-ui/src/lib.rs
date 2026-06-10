//! `nies-ui` — platform-agnostic UI/rendering shared by both frontends.
//!
//! Currently hosts [`NesRenderer`] (the wgpu palette-LUT renderer that blits
//! the PPU framebuffer to the screen) plus the palette and viewport-scaling
//! helpers. egui debugger panels and settings UI land in later milestones.

pub mod input;
pub mod pacing;
pub mod palette;
pub mod renderer;
pub mod scaling;
pub use renderer::NesRenderer;
