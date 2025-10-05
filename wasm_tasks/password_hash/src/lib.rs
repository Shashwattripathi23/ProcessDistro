/// Password Hashing WASM Task
/// 
/// CPU-intensive password hashing task for benchmarking and real workloads
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct HashInput {
    passwords: Vec<String>,
    salt: String,
    algorithm: HashAlgorithm,
    iterations: Option<u32>,
}

#[derive(Deserialize)]
enum HashAlgorithm {
    Bcrypt,
    Sha256,
    Sha512,
}

#[derive(Serialize)]
struct HashOutput {
    hashed_passwords: Vec<String>,
    total_time_ms: u64,
    passwords_per_second: f64,
}

#[no_mangle]
pub extern "C" fn hash_passwords(input_ptr: *const u8, input_len: usize) -> *const u8 {
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    
    match process_hashing(input_slice) {
        Ok(output) => {
            let output_json = serde_json::to_string(&output).unwrap();
            let output_bytes = output_json.into_bytes();
            let boxed_slice = output_bytes.into_boxed_slice();
            Box::into_raw(boxed_slice) as *const u8
        }
        Err(_) => std::ptr::null()
    }
}

fn process_hashing(input_data: &[u8]) -> Result<HashOutput, Box<dyn std::error::Error>> {
    let input: HashInput = serde_json::from_slice(input_data)?;
    let start_time = std::time::Instant::now();
    
    let mut hashed_passwords = Vec::new();
    
    for password in &input.passwords {
        let hashed = match input.algorithm {
            HashAlgorithm::Bcrypt => {
                let cost = input.iterations.unwrap_or(10);
                bcrypt::hash(password, cost)?
            }
            HashAlgorithm::Sha256 => {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(password.as_bytes());
                hasher.update(input.salt.as_bytes());
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Sha512 => {
                use sha2::{Sha512, Digest};
                let mut hasher = Sha512::new();
                hasher.update(password.as_bytes());
                hasher.update(input.salt.as_bytes());
                format!("{:x}", hasher.finalize())
            }
        };
        
        hashed_passwords.push(hashed);
    }
    
    let elapsed = start_time.elapsed();
    let total_time_ms = elapsed.as_millis() as u64;
    let passwords_per_second = if total_time_ms > 0 {
        (input.passwords.len() as f64) / (total_time_ms as f64 / 1000.0)
    } else {
        0.0
    };
    
    Ok(HashOutput {
        hashed_passwords,
        total_time_ms,
        passwords_per_second,
    })
}

#[no_mangle]
pub extern "C" fn main() {
    // Entry point for WASI execution
    // Read input from stdin, process, write to stdout
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    
    let result = process_hashing(input.as_bytes());
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