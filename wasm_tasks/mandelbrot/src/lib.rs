/// Mandelbrot Set Renderer WASM Task
/// 
/// Embarrassingly parallel fractal generation
use serde::{Deserialize, Serialize};
use num_complex::Complex;

#[derive(Deserialize)]
struct MandelbrotInput {
    width: u32,
    height: u32,
    tile_x: u32,
    tile_y: u32,
    tile_width: u32,
    tile_height: u32,
    min_real: f64,
    max_real: f64,
    min_imag: f64,
    max_imag: f64,
    max_iterations: u32,
}

#[derive(Serialize)]
struct MandelbrotOutput {
    tile_data: Vec<u32>,
    tile_x: u32,
    tile_y: u32,
    tile_width: u32,
    tile_height: u32,
    computation_time_ms: u64,
    pixels_calculated: u32,
}

#[no_mangle]
pub extern "C" fn render_mandelbrot_tile(input_ptr: *const u8, input_len: usize) -> *const u8 {
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    
    match process_mandelbrot(input_slice) {
        Ok(output) => {
            let output_json = serde_json::to_string(&output).unwrap();
            let output_bytes = output_json.into_bytes();
            let boxed_slice = output_bytes.into_boxed_slice();
            Box::into_raw(boxed_slice) as *const u8
        }
        Err(_) => std::ptr::null()
    }
}

fn process_mandelbrot(input_data: &[u8]) -> Result<MandelbrotOutput, Box<dyn std::error::Error>> {
    let input: MandelbrotInput = serde_json::from_slice(input_data)?;
    let start_time = std::time::Instant::now();
    
    let mut tile_data = Vec::with_capacity((input.tile_width * input.tile_height) as usize);
    
    // Calculate scaling factors
    let real_scale = (input.max_real - input.min_real) / input.width as f64;
    let imag_scale = (input.max_imag - input.min_imag) / input.height as f64;
    
    for pixel_y in 0..input.tile_height {
        for pixel_x in 0..input.tile_width {
            // Calculate the actual coordinates
            let actual_x = input.tile_x + pixel_x;
            let actual_y = input.tile_y + pixel_y;
            
            // Map pixel to complex plane
            let real = input.min_real + actual_x as f64 * real_scale;
            let imag = input.min_imag + actual_y as f64 * imag_scale;
            let c = Complex::new(real, imag);
            
            // Calculate iterations for this point
            let iterations = mandelbrot_iterations(c, input.max_iterations);
            
            // Convert iterations to color value
            let color = iterations_to_color(iterations, input.max_iterations);
            tile_data.push(color);
        }
    }
    
    let elapsed = start_time.elapsed();
    let computation_time_ms = elapsed.as_millis() as u64;
    
    Ok(MandelbrotOutput {
        tile_data,
        tile_x: input.tile_x,
        tile_y: input.tile_y,
        tile_width: input.tile_width,
        tile_height: input.tile_height,
        computation_time_ms,
        pixels_calculated: input.tile_width * input.tile_height,
    })
}

fn mandelbrot_iterations(c: Complex<f64>, max_iterations: u32) -> u32 {
    let mut z = Complex::new(0.0, 0.0);
    let mut iterations = 0;
    
    while iterations < max_iterations && z.norm_sqr() <= 4.0 {
        z = z * z + c;
        iterations += 1;
    }
    
    iterations
}

fn iterations_to_color(iterations: u32, max_iterations: u32) -> u32 {
    if iterations == max_iterations {
        // Point is in the Mandelbrot set - black
        0x000000FF
    } else {
        // Color based on iteration count
        let ratio = iterations as f32 / max_iterations as f32;
        
        // Simple color gradient: blue to red to yellow
        let r = (255.0 * (ratio * 2.0).min(1.0)) as u8;
        let g = (255.0 * ((ratio - 0.5) * 2.0).max(0.0)) as u8;
        let b = (255.0 * (1.0 - ratio)) as u8;
        
        // RGBA format
        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xFF
    }
}

#[no_mangle]
pub extern "C" fn main() {
    // Entry point for WASI execution
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    
    let result = process_mandelbrot(input.as_bytes());
    match result {
        Ok(output) => {
            let output_json = serde_json::to_string(&output).unwrap();
            println!("{}", output_json);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

// Advanced coloring algorithms
fn smooth_coloring(c: Complex<f64>, max_iterations: u32) -> u32 {
    let mut z = Complex::new(0.0, 0.0);
    let mut iterations = 0;
    
    while iterations < max_iterations && z.norm_sqr() <= 4.0 {
        z = z * z + c;
        iterations += 1;
    }
    
    if iterations < max_iterations {
        // Smooth coloring using continuous potential
        let log_zn = (z.norm_sqr()).ln() / 2.0;
        let nu = (log_zn / 2.0_f64.ln()).ln() / 2.0_f64.ln();
        let smooth_iterations = iterations as f64 + 1.0 - nu;
        
        let ratio = smooth_iterations / max_iterations as f64;
        hsv_to_rgb(ratio * 360.0, 1.0, 1.0)
    } else {
        0x000000FF // Black for points in the set
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> u32 {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    
    let r = ((r + m) * 255.0) as u8;
    let g = ((g + m) * 255.0) as u8;
    let b = ((b + m) * 255.0) as u8;
    
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xFF
}