import * as vscode from 'vscode';
import * as path from 'path';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export async function startLspClient(context: vscode.ExtensionContext): Promise<boolean> {
    const config = vscode.workspace.getConfiguration('orthos');
    
    if (!config.get<boolean>('enableLsp')) {
        return false;
    }

    let lspPath = config.get<string>('lspPath');

    if (!lspPath) {
        // Auto-detect: look in workspace
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders) {
            const possiblePaths = [
                'target/release/orthos-lsp',
                'target/debug/orthos-lsp',
                'orthos-lsp/target/release/orthos-lsp',
                'orthos-lsp/target/debug/orthos-lsp',
            ];
            for (const p of possiblePaths) {
                const fullPath = vscode.Uri.joinPath(workspaceFolders[0].uri, p);
                try {
                    await vscode.workspace.fs.stat(fullPath);
                    lspPath = fullPath.fsPath;
                    break;
                } catch {
                    // Not found, try next
                }
            }
        }
    }

    if (!lspPath) {
        console.log('orthos-lsp not found, LSP features disabled');
        return false;
    }

    const serverOptions: ServerOptions = {
        run: {
            command: lspPath,
            transport: TransportKind.stdio,
        },
        debug: {
            command: lspPath,
            transport: TransportKind.stdio,
        },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'orthos' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.orth'),
        },
    };

    client = new LanguageClient(
        'orthos-lsp',
        'Orthos Language Server',
        serverOptions,
        clientOptions
    );

    await client.start();
    console.log('Orthos LSP client started');
    return true;
}

export async function stopLspClient(): Promise<void> {
    if (client) {
        await client.stop();
        client = undefined;
    }
}

export function isLspRunning(): boolean {
    return client !== undefined && client.isRunning();
}
