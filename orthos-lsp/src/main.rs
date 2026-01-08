use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use orthos_kernel::{parse, solve_program, FluxValue, SolveResult};

#[derive(Debug)]
struct DocumentState {
    text: String,
    solve_result: Option<SolveResult>,
    flux_locations: Vec<FluxLocation>,
}

#[derive(Debug, Clone)]
struct FluxLocation {
    name: String,
    qualified_name: String,
    line: u32,
}

struct OrthosBackend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
}

impl OrthosBackend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn analyze_document(&self, uri: &Url, text: &str) {
        let (solve_result, flux_locations, diagnostics) = self.run_analysis(text);

        // Store state
        {
            let mut docs = self.documents.write().await;
            docs.insert(
                uri.clone(),
                DocumentState {
                    text: text.to_string(),
                    solve_result: Some(solve_result),
                    flux_locations,
                },
            );
        }

        // Publish diagnostics
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn run_analysis(&self, text: &str) -> (SolveResult, Vec<FluxLocation>, Vec<Diagnostic>) {
        let mut diagnostics = Vec::new();
        let mut flux_locations = Vec::new();

        // Parse the document
        let program = match parse(text) {
            Ok(p) => p,
            Err(e) => {
                // Parse error - create diagnostic at start of file
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 1 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("orthos".to_string()),
                    message: format!("Parse error: {}", e),
                    related_information: None,
                    tags: None,
                    data: None,
                });
                return (
                    SolveResult::error(e),
                    flux_locations,
                    diagnostics,
                );
            }
        };

        // Extract flux locations for inlay hints
        for boundary in &program.boundaries {
            self.extract_flux_locations(text, boundary, "", &mut flux_locations);
        }

        // Solve the program
        let result = solve_program(&program);

        // Generate diagnostics based on solve result
        if result.status == "UNSAT" {
            if let Some(ref core) = result.unsat_core {
                // Find the laws that caused UNSAT and map to line numbers
                for law_name in core {
                    if let Some(line) = self.find_law_line(text, law_name) {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position { line, character: 0 },
                                end: Position { line, character: 100 },
                            },
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: None,
                            code_description: None,
                            source: Some("orthos".to_string()),
                            message: format!("UNSAT: Constraint '{}' contributes to contradiction", law_name),
                            related_information: None,
                            tags: None,
                            data: None,
                        });
                    }
                }

                // If no specific laws found, show general UNSAT error
                if diagnostics.is_empty() {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: 1 },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("orthos".to_string()),
                        message: format!("UNSAT: No solution exists. Conflicting laws: {:?}", core),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
        } else if result.status == "ERROR" {
            if let Some(ref err) = result.error {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 1 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("orthos".to_string()),
                    message: format!("Solver error: {}", err),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        (result, flux_locations, diagnostics)
    }

    fn extract_flux_locations(
        &self,
        text: &str,
        boundary: &orthos_kernel::Boundary,
        prefix: &str,
        locations: &mut Vec<FluxLocation>,
    ) {
        let boundary_prefix = if prefix.is_empty() {
            boundary.name.clone()
        } else {
            format!("{}_{}", prefix, boundary.name)
        };

        for flux in &boundary.flux {
            // Find the line where this flux is declared
            if let Some(line) = self.find_flux_line(text, &flux.name) {
                locations.push(FluxLocation {
                    name: flux.name.clone(),
                    qualified_name: format!("{}_{}", boundary_prefix, flux.name),
                    line,
                });
            }
        }

        // Process nested boundaries
        for nested in &boundary.nested_boundaries {
            self.extract_flux_locations(text, nested, &boundary_prefix, locations);
        }
    }

    fn find_flux_line(&self, text: &str, flux_name: &str) -> Option<u32> {
        for (line_num, line) in text.lines().enumerate() {
            if line.contains("Flux") && line.contains(flux_name) {
                return Some(line_num as u32);
            }
        }
        None
    }

    fn find_law_line(&self, text: &str, law_name: &str) -> Option<u32> {
        for (line_num, line) in text.lines().enumerate() {
            if (line.contains("Law") || line.contains("Goal")) && line.contains(law_name) {
                return Some(line_num as u32);
            }
        }
        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for OrthosBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                inlay_hint_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "orthos-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Orthos LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.analyze_document(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.analyze_document(&uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut docs = self.documents.write().await;
        docs.remove(&params.text_document.uri);
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let docs = self.documents.read().await;
        let uri = &params.text_document.uri;

        let Some(doc) = docs.get(uri) else {
            return Ok(None);
        };

        let Some(ref result) = doc.solve_result else {
            return Ok(None);
        };

        if result.status != "SAT" {
            return Ok(None);
        }

        let Some(ref model) = result.model else {
            return Ok(None);
        };

        let mut hints = Vec::new();

        for flux_loc in &doc.flux_locations {
            if let Some(value) = model.get(&flux_loc.qualified_name) {
                let value_str = format_flux_value(value);
                
                // Find the end of the line
                let line_text = doc.text.lines().nth(flux_loc.line as usize).unwrap_or("");
                let end_char = line_text.len() as u32;

                hints.push(InlayHint {
                    position: Position {
                        line: flux_loc.line,
                        character: end_char,
                    },
                    label: InlayHintLabel::String(format!(" = {}", value_str)),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: Some(InlayHintTooltip::String(format!(
                        "Solved value for {}",
                        flux_loc.name
                    ))),
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                });
            }
        }

        Ok(Some(hints))
    }
}

fn format_flux_value(value: &FluxValue) -> String {
    match value {
        FluxValue::Int(n) => n.to_string(),
        FluxValue::Bool(b) => b.to_string(),
        FluxValue::String(s) => format!("\"{}\"", s),
        FluxValue::List(items) => {
            let items_str: Vec<String> = items.iter().map(format_flux_value).collect();
            format!("[{}]", items_str.join(", "))
        }
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(OrthosBackend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
