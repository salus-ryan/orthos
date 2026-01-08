import * as vscode from 'vscode';

export class OrthosOutputChannel {
    private channel: vscode.OutputChannel;
    
    constructor() {
        this.channel = vscode.window.createOutputChannel('Orthos');
    }
    
    log(message: string): void {
        const timestamp = new Date().toLocaleTimeString();
        this.channel.appendLine(`[${timestamp}] ${message}`);
    }
    
    show(): void {
        this.channel.show();
    }
    
    clear(): void {
        this.channel.clear();
    }
    
    dispose(): void {
        this.channel.dispose();
    }
}
