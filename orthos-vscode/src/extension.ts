import * as vscode from 'vscode';
import { OrthosDaemon } from './daemon';
import { OrthosStatusBar } from './statusBar';
import { OrthosOutputChannel } from './output';
import { startLspClient, stopLspClient } from './lspClient';

let daemon: OrthosDaemon | undefined;
let statusBar: OrthosStatusBar;
let output: OrthosOutputChannel;
let lspStarted = false;

export async function activate(context: vscode.ExtensionContext) {
    output = new OrthosOutputChannel();
    statusBar = new OrthosStatusBar();
    
    output.log('Orthos extension activated');
    
    // Start LSP client for real-time diagnostics and inlay hints
    try {
        lspStarted = await startLspClient(context);
        if (lspStarted) {
            output.log('LSP client started - diagnostics and inlay hints enabled');
        }
    } catch (err) {
        output.log(`LSP client failed to start: ${err}`);
    }
    
    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('orthos.initialize', () => initializeCommand()),
        vscode.commands.registerCommand('orthos.checkpoint', () => checkpointCommand()),
        vscode.commands.registerCommand('orthos.restore', () => restoreCommand()),
        vscode.commands.registerCommand('orthos.constrain', () => constrainCommand()),
        vscode.commands.registerCommand('orthos.query', () => queryCommand()),
        vscode.commands.registerCommand('orthos.shutdown', () => shutdownCommand()),
    );
    
    // Auto-initialize on .orth file open
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument((doc) => {
            if (doc.languageId === 'orthos') {
                const config = vscode.workspace.getConfiguration('orthos');
                if (config.get('autoInitialize') && !daemon?.isRunning()) {
                    initializeCommand();
                }
            }
        })
    );
    
    // Clean up on deactivate
    context.subscriptions.push({
        dispose: () => {
            daemon?.shutdown();
            statusBar.dispose();
            output.dispose();
        }
    });
    
    statusBar.setStatus('inactive', 'Orthos: Inactive');
}

async function initializeCommand() {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'orthos') {
        vscode.window.showWarningMessage('Open an .orth file first');
        return;
    }
    
    const source = editor.document.getText();
    const fileName = editor.document.fileName;
    
    output.log(`Initializing universe from ${fileName}...`);
    statusBar.setStatus('loading', 'Orthos: Initializing...');
    
    try {
        // Find daemon path
        const config = vscode.workspace.getConfiguration('orthos');
        let daemonPath = config.get<string>('daemonPath');
        
        if (!daemonPath) {
            // Auto-detect: look in workspace
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (workspaceFolders) {
                const possiblePaths = [
                    'target/release/orthos-daemon',
                    'target/debug/orthos-daemon',
                    'orthos-daemon/target/release/orthos-daemon',
                    'orthos-daemon/target/debug/orthos-daemon',
                ];
                for (const p of possiblePaths) {
                    const fullPath = vscode.Uri.joinPath(workspaceFolders[0].uri, p);
                    try {
                        await vscode.workspace.fs.stat(fullPath);
                        daemonPath = fullPath.fsPath;
                        break;
                    } catch {
                        // Not found, try next
                    }
                }
            }
        }
        
        if (!daemonPath) {
            vscode.window.showErrorMessage('orthos-daemon not found. Set orthos.daemonPath in settings.');
            statusBar.setStatus('error', 'Orthos: Daemon not found');
            return;
        }
        
        // Shutdown existing daemon
        if (daemon?.isRunning()) {
            await daemon.shutdown();
        }
        
        // Start new daemon
        daemon = new OrthosDaemon(daemonPath, output);
        await daemon.start();
        
        // Initialize with source
        const result = await daemon.initialize(source);
        
        if (result.error) {
            output.log(`ERROR: ${result.error.message}`);
            if (result.error.core) {
                output.log(`Conflicts: ${result.error.core.join(', ')}`);
            }
            statusBar.setStatus('error', 'Orthos: UNSAT');
            vscode.window.showErrorMessage(`Ontological Error: ${result.error.message}`);
            return;
        }
        
        const fluxList = result.result?.flux_list || [];
        output.log(`Universe is SAT. Flux variables: ${fluxList.join(', ')}`);
        statusBar.setStatus('ready', `Orthos: SAT (${fluxList.length} flux)`);
        vscode.window.showInformationMessage(`Orthos initialized: ${fluxList.length} flux variables`);
        
    } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        output.log(`ERROR: ${msg}`);
        statusBar.setStatus('error', 'Orthos: Error');
        vscode.window.showErrorMessage(`Failed to initialize: ${msg}`);
    }
}

async function checkpointCommand() {
    if (!daemon?.isRunning()) {
        vscode.window.showWarningMessage('Daemon not running. Initialize first.');
        return;
    }
    
    try {
        const result = await daemon.checkpoint();
        if (result.error) {
            output.log(`Checkpoint failed: ${result.error.message}`);
            return;
        }
        output.log('Checkpoint created');
        statusBar.setStatus('checkpoint', 'Orthos: Checkpoint');
        vscode.window.showInformationMessage('Checkpoint created');
    } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        output.log(`ERROR: ${msg}`);
    }
}

async function restoreCommand() {
    if (!daemon?.isRunning()) {
        vscode.window.showWarningMessage('Daemon not running. Initialize first.');
        return;
    }
    
    try {
        const result = await daemon.restore();
        if (result.error) {
            output.log(`Restore failed: ${result.error.message}`);
            vscode.window.showErrorMessage(result.error.message);
            return;
        }
        output.log('Restored to checkpoint');
        statusBar.setStatus('ready', 'Orthos: SAT');
        vscode.window.showInformationMessage('Restored to checkpoint');
    } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        output.log(`ERROR: ${msg}`);
    }
}

async function constrainCommand() {
    if (!daemon?.isRunning()) {
        vscode.window.showWarningMessage('Daemon not running. Initialize first.');
        return;
    }
    
    const input = await vscode.window.showInputBox({
        prompt: 'Enter constraint (e.g., "Boundary.flux > 10")',
        placeHolder: 'Main.x == 42'
    });
    
    if (!input) {
        return;
    }
    
    try {
        output.log(`Constraining: ${input}`);
        const result = await daemon.constrain([input]);
        
        if (result.result?.status === 'UNSAT') {
            output.log('UNSAT - Constraint rejected (auto-rollback)');
            statusBar.setStatus('unsat', 'Orthos: UNSAT');
            vscode.window.showWarningMessage('UNSAT: Constraint would create contradiction');
            return;
        }
        
        output.log('SAT - Constraint accepted');
        statusBar.setStatus('ready', 'Orthos: SAT');
        vscode.window.showInformationMessage('Constraint accepted');
    } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        output.log(`ERROR: ${msg}`);
    }
}

async function queryCommand() {
    if (!daemon?.isRunning()) {
        vscode.window.showWarningMessage('Daemon not running. Initialize first.');
        return;
    }
    
    const input = await vscode.window.showInputBox({
        prompt: 'Enter flux names to query (comma-separated)',
        placeHolder: 'Main.x, Main.y'
    });
    
    if (!input) {
        return;
    }
    
    const fluxNames = input.split(',').map(s => s.trim()).filter(s => s);
    
    try {
        output.log(`Querying: ${fluxNames.join(', ')}`);
        const result = await daemon.query(fluxNames);
        
        if (result.error) {
            output.log(`Query failed: ${result.error.message}`);
            vscode.window.showErrorMessage(result.error.message);
            return;
        }
        
        const values = result.result?.values || {};
        const lines = Object.entries(values).map(([k, v]) => `  ${k} = ${JSON.stringify(v)}`);
        output.log('Query results:\n' + lines.join('\n'));
        
        // Show in info message
        const summary = Object.entries(values).map(([k, v]) => `${k}=${JSON.stringify(v)}`).join(', ');
        vscode.window.showInformationMessage(`Query: ${summary}`);
    } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        output.log(`ERROR: ${msg}`);
    }
}

async function shutdownCommand() {
    if (!daemon?.isRunning()) {
        vscode.window.showInformationMessage('Daemon not running');
        return;
    }
    
    try {
        await daemon.shutdown();
        output.log('Daemon shutdown');
        statusBar.setStatus('inactive', 'Orthos: Inactive');
        vscode.window.showInformationMessage('Orthos daemon stopped');
    } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        output.log(`ERROR: ${msg}`);
    }
}

export async function deactivate() {
    await stopLspClient();
    daemon?.shutdown();
}
