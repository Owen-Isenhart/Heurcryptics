use hilbert_curve::convert_1d_to_2d;
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnalysisReport {
    pub file_name: String,
    pub file_type: String,
    pub entropy: f64,
    pub byte_freq: Vec<f64>,
    pub markov_top_transitions: HashMap<String, f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MappingMetadata {
    pub files: HashMap<String, usize>, 
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "train" => {
            if args.len() < 4 {
                println!("Usage: cargo run -- train <dir_path> <label>");
                return;
            }
            handle_train(&args[2], &args[3]);
        }
        "train-all" => {
            handle_train_all();
        }
        "identify" => {
            if args.len() < 3 {
                println!("Usage: cargo run -- identify <file_path>");
                return;
            }
            handle_identify(&args[2]);
        }
        "reconstruct" => {
            if args.len() < 3 {
                println!("Usage: cargo run -- reconstruct <file_path> <output_path>");
                return;
            }
            handle_reconstruct(&args[2], &args[3]);
        }
        _ => print_usage(),
    }
}

fn print_usage() {
    println!("Hilbert Analytics & Forensic Engine");
    println!("----------------------------------");
    println!("Commands:");
    println!("  train <dir> <label>       Analyze specific directory");
    println!("  train-all                 Analyze all subdirectories in ./samples/");
    println!("  identify <file>           Guess file type");
    println!("  reconstruct <file> <out>   Restore file from map");
}

// --- NEW: TRAIN-ALL LOGIC ---

fn handle_train_all() {
    let samples_dir = Path::new("samples");
    if !samples_dir.exists() {
        println!("Error: 'samples' directory not found.");
        return;
    }

    let entries = fs::read_dir(samples_dir).expect("Could not read samples directory");
    
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Use the folder name as the label
            let label = path.file_name().unwrap().to_str().unwrap();
            println!("\n--- Automatically training category: {} ---", label);
            handle_train(path.to_str().unwrap(), label);
        }
    }
    println!("\n[Batch Training Complete]");
}

// --- TRAINING LOGIC (Modified to be called by train-all) ---

fn handle_train(dir_path: &str, label: &str) {
    let mut current_dataset = load_global_dataset();
    let category_dir = Path::new("maps").join(label);
    
    if !category_dir.exists() {
        fs::create_dir_all(&category_dir).expect("Failed to create category directory");
    }

    let paths = fs::read_dir(dir_path).expect("Target directory not found");

    for path in paths.flatten() {
        let file_path = path.path();
        if file_path.is_file() {
            let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();

            // Duplicate Check
            if current_dataset.iter().any(|r| r.file_name == file_name && r.file_type == label) {
                println!("  Skipping {}: Already exists in dataset.", file_name);
                continue;
            }

            println!("  Processing: {}", file_name);
            let data = fs::read(&file_path).expect("Could not read file");
            let report = analyze_bytes(label.to_string(), file_name.clone(), &data);
            current_dataset.push(report);
            save_hilbert_map(&file_path, &data, label);
        }
    }

    let json = serde_json::to_string_pretty(&current_dataset).unwrap();
    fs::write("global_dataset.json", json).expect("Unable to write dataset");
}

// --- IDENTIFYING LOGIC ---

fn handle_identify(file_path: &str) {
    let dataset = load_global_dataset();
    if dataset.is_empty() {
        println!("Error: global_dataset.json is empty. Run train-all first!");
        return;
    }

    let data = fs::read(file_path).expect("Could not read target file");
    let mystery = analyze_bytes("unknown".to_string(), "mystery".to_string(), &data);

    let mut scores: HashMap<String, Vec<f64>> = HashMap::new();

    for sample in dataset {
        let mut score = 0.0;
        let entropy_diff = (sample.entropy - mystery.entropy).abs();
        score += entropy_diff * 5.0;

        let mut markov_matches = 0;
        for (pair, _) in &mystery.markov_top_transitions {
            if sample.markov_top_transitions.contains_key(pair) {
                markov_matches += 1;
            }
        }
        score -= (markov_matches as f64) * 2.0;

        let null_diff = (sample.byte_freq[0] - mystery.byte_freq[0]).abs();
        score += null_diff * 10.0;

        scores.entry(sample.file_type).or_insert(Vec::new()).push(score);
    }

    let mut final_results: Vec<(String, f64)> = scores.iter()
        .map(|(cat, list)| {
            let avg: f64 = list.iter().sum::<f64>() / list.len() as f64;
            (cat.clone(), avg)
        })
        .collect();

    final_results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("\n--- IDENTIFICATION RESULTS ---");
    for (cat, score) in final_results.iter().take(3) {
        println!("Category: {:<12} | Score: {:.4}", cat, score);
    }
}

// --- RECONSTRUCTION LOGIC ---

fn handle_reconstruct(img_path: &str, output_path: &str) {
    let metadata = load_metadata();
    let file_name = Path::new(img_path).file_name().unwrap().to_str().unwrap().replace(".png", "");
    
    let original_len = match metadata.files.get(&file_name) {
        Some(&len) => len,
        None => {
            println!("Error: No metadata for {}.", file_name);
            return;
        }
    };

    let img = image::open(img_path).expect("Failed to open image").to_luma8();
    let (width, _) = img.dimensions();
    let n = width as usize;

    let mut reconstructed_bytes = Vec::with_capacity(original_len);
    for i in 0..original_len {
        let (x, y) = convert_1d_to_2d(i, n);
        let pixel = img.get_pixel(x as u32, y as u32);
        reconstructed_bytes.push(pixel[0]);
    }

    fs::write(output_path, reconstructed_bytes).expect("Failed to write file");
    println!("Reconstruction Successful: Saved to {}", output_path);
}

// --- UTILITIES ---

fn analyze_bytes(label: String, name: String, data: &[u8]) -> AnalysisReport {
    let len = data.len() as f64;
    let mut counts = [0usize; 256];
    let mut transitions = HashMap::<(u8, u8), usize>::new();

    for i in 0..data.len() {
        counts[data[i] as usize] += 1;
        if i < data.len() - 1 {
            let pair = (data[i], data[i+1]);
            *transitions.entry(pair).or_insert(0) += 1;
        }
    }

    let mut entropy = 0.0;
    let mut byte_freq = Vec::with_capacity(256);
    for count in counts.iter() {
        let p = (*count as f64) / len;
        byte_freq.push(p);
        if p > 0.0 { entropy -= p * p.log2(); }
    }

    let mut markov_sorted: Vec<_> = transitions.iter().collect();
    markov_sorted.sort_by(|a, b| b.1.cmp(a.1));
    let top_patterns = markov_sorted.into_iter().take(20)
        .map(|(pair, count)| (format!("{:02x}{:02x}", pair.0, pair.1), (*count as f64) / len))
        .collect();

    AnalysisReport { 
        file_name: name,
        file_type: label, 
        entropy, 
        byte_freq, 
        markov_top_transitions: top_patterns 
    }
}

fn save_hilbert_map(path: &Path, data: &[u8], label: &str) {
    let data_len = data.len();
    let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
    let output_dir = Path::new("maps").join(label);
    if !output_dir.exists() { fs::create_dir_all(&output_dir).ok(); }

    let order = ((data_len as f64).log2() / 2.0).ceil() as u32;
    let n = 2usize.pow(order);
    
    let mut img = ImageBuffer::new(n as u32, n as u32);
    for (i, &byte) in data.iter().enumerate() {
        let (x, y) = convert_1d_to_2d(i, n);
        img.put_pixel(x as u32, y as u32, Luma([byte]));
    }
    
    img.save(output_dir.join(format!("{}.png", file_name))).ok();

    let mut metadata = load_metadata();
    metadata.files.insert(file_name, data_len);
    let json = serde_json::to_string_pretty(&metadata).unwrap();
    fs::write("mapping_metadata.json", json).ok();
}

fn load_global_dataset() -> Vec<AnalysisReport> {
    fs::read_to_string("global_dataset.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_metadata() -> MappingMetadata {
    fs::read_to_string("mapping_metadata.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| MappingMetadata { files: HashMap::new() })
}