use orthos_kernel::{parse, solve_program, Transpiler};

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("ORTHOS Kernel v0.1");
        eprintln!("Usage: {} <file.orth> [--smt]", args[0]);
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --smt    Output SMT-LIB2 script instead of solving");
        process::exit(1);
    }
    
    let filename = &args[1];
    let output_smt = args.iter().any(|a| a == "--smt");
    
    // Read source file
    let source = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };
    
    // Parse
    let program = match parse(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };
    
    if output_smt {
        // Output SMT-LIB2 script
        let mut transpiler = Transpiler::new();
        match transpiler.transpile(&program) {
            Ok(smt) => println!("{}", smt),
            Err(e) => {
                eprintln!("Transpile error: {}", e);
                process::exit(1);
            }
        }
    } else {
        // Solve and output JSON
        let result = solve_program(&program);
        println!("{}", result.to_json());
        
        // Exit with appropriate code
        match result.status.as_str() {
            "SAT" => process::exit(0),
            "UNSAT" => process::exit(1),
            _ => process::exit(2),
        }
    }
}
