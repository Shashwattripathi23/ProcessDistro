/// Matrix Multiplication WASM Task
/// 
/// Parallelizable matrix multiplication with tile-based computation
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct MatrixMulInput {
    matrix_a: Matrix,
    matrix_b: Matrix,
    tile_row: usize,
    tile_col: usize,
    tile_size: usize,
}

#[derive(Deserialize, Serialize)]
struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

#[derive(Serialize)]
struct MatrixMulOutput {
    result_tile: Matrix,
    computation_time_ms: u64,
    operations_count: u64,
}

impl Matrix {
    fn get(&self, row: usize, col: usize) -> f64 {
        if row >= self.rows || col >= self.cols {
            0.0
        } else {
            self.data[row * self.cols + col]
        }
    }
    
    fn set(&mut self, row: usize, col: usize, value: f64) {
        if row < self.rows && col < self.cols {
            self.data[row * self.cols + col] = value;
        }
    }
    
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }
}

#[no_mangle]
pub extern "C" fn multiply_matrix_tile(input_ptr: *const u8, input_len: usize) -> *const u8 {
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    
    match process_matrix_multiplication(input_slice) {
        Ok(output) => {
            let output_json = serde_json::to_string(&output).unwrap();
            let output_bytes = output_json.into_bytes();
            let boxed_slice = output_bytes.into_boxed_slice();
            Box::into_raw(boxed_slice) as *const u8
        }
        Err(_) => std::ptr::null()
    }
}

fn process_matrix_multiplication(input_data: &[u8]) -> Result<MatrixMulOutput, Box<dyn std::error::Error>> {
    let input: MatrixMulInput = serde_json::from_slice(input_data)?;
    let start_time = std::time::Instant::now();
    
    // Calculate the actual tile boundaries
    let start_row = input.tile_row * input.tile_size;
    let end_row = ((input.tile_row + 1) * input.tile_size).min(input.matrix_a.rows);
    let start_col = input.tile_col * input.tile_size;
    let end_col = ((input.tile_col + 1) * input.tile_size).min(input.matrix_b.cols);
    
    let result_rows = end_row - start_row;
    let result_cols = end_col - start_col;
    let mut result_tile = Matrix::new(result_rows, result_cols);
    
    let mut operations = 0u64;
    
    // Perform matrix multiplication for this tile
    for i in 0..result_rows {
        for j in 0..result_cols {
            let mut sum = 0.0;
            
            // Dot product of row i from matrix A and column j from matrix B
            for k in 0..input.matrix_a.cols {
                let a_val = input.matrix_a.get(start_row + i, k);
                let b_val = input.matrix_b.get(k, start_col + j);
                sum += a_val * b_val;
                operations += 1;
            }
            
            result_tile.set(i, j, sum);
        }
    }
    
    let elapsed = start_time.elapsed();
    let computation_time_ms = elapsed.as_millis() as u64;
    
    Ok(MatrixMulOutput {
        result_tile,
        computation_time_ms,
        operations_count: operations,
    })
}

#[no_mangle]
pub extern "C" fn main() {
    // Entry point for WASI execution
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    
    let result = process_matrix_multiplication(input.as_bytes());
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

// Optimized matrix multiplication for larger matrices
fn multiply_matrices_blocked(a: &Matrix, b: &Matrix, block_size: usize) -> Matrix {
    let mut result = Matrix::new(a.rows, b.cols);
    
    for i_block in (0..a.rows).step_by(block_size) {
        for j_block in (0..b.cols).step_by(block_size) {
            for k_block in (0..a.cols).step_by(block_size) {
                // Multiply blocks
                let i_end = (i_block + block_size).min(a.rows);
                let j_end = (j_block + block_size).min(b.cols);
                let k_end = (k_block + block_size).min(a.cols);
                
                for i in i_block..i_end {
                    for j in j_block..j_end {
                        let mut sum = result.get(i, j);
                        for k in k_block..k_end {
                            sum += a.get(i, k) * b.get(k, j);
                        }
                        result.set(i, j, sum);
                    }
                }
            }
        }
    }
    
    result
}