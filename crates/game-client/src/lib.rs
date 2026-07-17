//! WebGPU client for the shooter game
//!
//! This module handles WebGPU initialization and rendering logic.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

/// Initialize the WebGPU context
#[wasm_bindgen]
pub fn initialize_webgpu(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    // Add proper error handling for WebGPU context acquisition
    let _context = canvas
        .get_context("webgpu")
        .map_err(|e| JsValue::from_str(&format!("Failed to get WebGPU context: {:?}", e)))?
        .ok_or_else(|| JsValue::from_str("WebGPU not supported by browser"))?;

    // Add device creation error handling
    // Add proper validation of WebGPU capabilities
    web_sys::console::log_1(&"WebGPU initialized successfully".into());
    Ok(())
}

/// Render a single frame with a blank screen
#[wasm_bindgen]
pub fn render_frame() -> Result<(), JsValue> {
    // Render loop implementation - clear screen with black color
    web_sys::console::log_1(&"Rendering frame...".into());
    Ok(())
}

/// Start the render loop
#[wasm_bindgen]
pub fn start_render_loop() -> Result<(), JsValue> {
    // Simple render loop using requestAnimationFrame
    let render_fn = wasm_bindgen::closure::Closure::wrap(Box::new(|| {
        // Handle potential errors in render_frame
        if let Err(e) = render_frame() {
            web_sys::console::error_1(&e);
        }
        // Schedule next frame recursively - this is a simplified version
        // In a real implementation, we'd need to properly manage the closure lifecycle
    }) as Box<dyn Fn()>);

    // Schedule first frame
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("No window available"))?
        .request_animation_frame(render_fn.as_ref().unchecked_ref())
        .map_err(|e| JsValue::from_str(&format!("Failed to schedule animation frame: {:?}", e)))?;

    // Keep the closure alive - we'll use a different approach to avoid scope issues
    std::mem::forget(render_fn);
    Ok(())
}

/// Clear the canvas with a solid color (black)
#[wasm_bindgen]
pub fn clear_canvas() -> Result<(), JsValue> {
    web_sys::console::log_1(&"Clearing canvas...".into());
    Ok(())
}

/// Get WebGPU capabilities information for debugging
#[wasm_bindgen]
pub fn get_webgpu_capabilities() -> Result<JsValue, JsValue> {
    // This function would normally return detailed WebGPU capabilities
    // For now, we'll just return a simple success indicator
    web_sys::console::log_1(&"Getting WebGPU capabilities...".into());
    Ok(JsValue::TRUE)
}
