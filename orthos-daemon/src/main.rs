mod protocol;
mod state;

use std::io::{self, BufRead, Write};

use protocol::{Request, Response};
use state::DaemonState;

fn main() {
    eprintln!("ORTHOS Daemon v0.1 - Oracle Server");
    eprintln!("Awaiting JSON-RPC commands on stdin...");
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    let mut state: Option<DaemonState> = None;
    
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading stdin: {}", e);
                continue;
            }
        };
        
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }
        
        // Parse request
        let request: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let error_response = Response::error(
                    None,
                    -32700,
                    format!("Parse error: {}", e),
                    None,
                );
                let _ = writeln!(stdout, "{}", serde_json::to_string(&error_response).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };
        
        // Dispatch method
        let response = dispatch(&mut state, request);
        
        // Write response
        let json = serde_json::to_string(&response).unwrap();
        let _ = writeln!(stdout, "{}", json);
        let _ = stdout.flush();
    }
}

fn dispatch(state: &mut Option<DaemonState>, request: Request) -> Response {
    let id = request.id;
    
    match request.method.as_str() {
        "initialize" => handle_initialize(state, request.params, id),
        "checkpoint" => handle_checkpoint(state, id),
        "constrain" => handle_constrain(state, request.params, id),
        "query" => handle_query(state, request.params, id),
        "restore" => handle_restore(state, id),
        "shutdown" => {
            eprintln!("Shutdown requested. Exiting.");
            std::process::exit(0);
        }
        _ => Response::error(
            id,
            -32601,
            format!("Method not found: {}", request.method),
            None,
        ),
    }
}

fn handle_initialize(
    state: &mut Option<DaemonState>,
    params: Option<serde_json::Value>,
    id: Option<u64>,
) -> Response {
    // Extract source from params
    let source = match params {
        Some(p) => match p.get("source") {
            Some(s) => match s.as_str() {
                Some(src) => src.to_string(),
                None => return Response::error(id, -32602, "Invalid params: source must be a string".to_string(), None),
            },
            None => return Response::error(id, -32602, "Invalid params: missing 'source' field".to_string(), None),
        },
        None => return Response::error(id, -32602, "Invalid params: params required".to_string(), None),
    };
    
    // Create new daemon state
    match DaemonState::new(&source) {
        Ok(new_state) => {
            let flux_list = new_state.get_flux_list();
            *state = Some(new_state);
            Response::success(id, serde_json::json!({
                "status": "READY",
                "flux_list": flux_list
            }))
        }
        Err(e) => {
            Response::error(id, -1, "Ontological Error".to_string(), Some(e.conflicts))
        }
    }
}

fn handle_checkpoint(state: &mut Option<DaemonState>, id: Option<u64>) -> Response {
    match state {
        Some(s) => {
            s.checkpoint();
            Response::success(id, serde_json::json!("OK"))
        }
        None => Response::error(id, -32002, "Not initialized".to_string(), None),
    }
}

fn handle_constrain(
    state: &mut Option<DaemonState>,
    params: Option<serde_json::Value>,
    id: Option<u64>,
) -> Response {
    let s = match state {
        Some(s) => s,
        None => return Response::error(id, -32002, "Not initialized".to_string(), None),
    };
    
    // Extract laws from params
    let laws: Vec<String> = match params {
        Some(p) => match p.get("laws") {
            Some(l) => match serde_json::from_value(l.clone()) {
                Ok(laws) => laws,
                Err(_) => return Response::error(id, -32602, "Invalid params: laws must be array of strings".to_string(), None),
            },
            None => return Response::error(id, -32602, "Invalid params: missing 'laws' field".to_string(), None),
        },
        None => return Response::error(id, -32602, "Invalid params: params required".to_string(), None),
    };
    
    match s.constrain(&laws) {
        Ok(()) => Response::success(id, serde_json::json!({ "status": "SAT" })),
        Err(e) => Response::success(id, serde_json::json!({
            "status": "UNSAT",
            "core": e.conflicts
        })),
    }
}

fn handle_query(
    state: &mut Option<DaemonState>,
    params: Option<serde_json::Value>,
    id: Option<u64>,
) -> Response {
    let s = match state {
        Some(s) => s,
        None => return Response::error(id, -32002, "Not initialized".to_string(), None),
    };
    
    // Extract flux names from params
    let flux_names: Vec<String> = match params {
        Some(p) => match p.get("flux") {
            Some(f) => match serde_json::from_value(f.clone()) {
                Ok(names) => names,
                Err(_) => return Response::error(id, -32602, "Invalid params: flux must be array of strings".to_string(), None),
            },
            None => return Response::error(id, -32602, "Invalid params: missing 'flux' field".to_string(), None),
        },
        None => return Response::error(id, -32602, "Invalid params: params required".to_string(), None),
    };
    
    match s.query(&flux_names) {
        Ok(values) => Response::success(id, serde_json::json!({ "values": values })),
        Err(e) => Response::error(id, -32003, e, None),
    }
}

fn handle_restore(state: &mut Option<DaemonState>, id: Option<u64>) -> Response {
    match state {
        Some(s) => {
            match s.restore() {
                Ok(()) => Response::success(id, serde_json::json!("OK")),
                Err(e) => Response::error(id, -32004, e, None),
            }
        }
        None => Response::error(id, -32002, "Not initialized".to_string(), None),
    }
}
