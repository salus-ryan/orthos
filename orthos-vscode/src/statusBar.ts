import * as vscode from 'vscode';

type StatusType = 'inactive' | 'loading' | 'ready' | 'checkpoint' | 'unsat' | 'error';

export class OrthosStatusBar {
    private statusBarItem: vscode.StatusBarItem;
    
    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.statusBarItem.show();
    }
    
    setStatus(status: StatusType, text: string): void {
        this.statusBarItem.text = this.getIcon(status) + ' ' + text;
        this.statusBarItem.backgroundColor = this.getBackgroundColor(status);
        this.statusBarItem.command = 'orthos.initialize';
    }
    
    private getIcon(status: StatusType): string {
        switch (status) {
            case 'inactive': return '$(circle-outline)';
            case 'loading': return '$(loading~spin)';
            case 'ready': return '$(check)';
            case 'checkpoint': return '$(bookmark)';
            case 'unsat': return '$(warning)';
            case 'error': return '$(error)';
            default: return '$(question)';
        }
    }
    
    private getBackgroundColor(status: StatusType): vscode.ThemeColor | undefined {
        switch (status) {
            case 'unsat':
            case 'error':
                return new vscode.ThemeColor('statusBarItem.errorBackground');
            case 'checkpoint':
                return new vscode.ThemeColor('statusBarItem.warningBackground');
            default:
                return undefined;
        }
    }
    
    dispose(): void {
        this.statusBarItem.dispose();
    }
}
