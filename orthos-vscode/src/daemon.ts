import { spawn, ChildProcess } from 'child_process';
import { OrthosOutputChannel } from './output';

interface DaemonResponse {
    result?: {
        status?: string;
        flux_list?: string[];
        values?: Record<string, unknown>;
        core?: string[];
    };
    error?: {
        code: number;
        message: string;
        core?: string[];
    };
    id?: number;
}

export class OrthosDaemon {
    private process: ChildProcess | null = null;
    private requestId = 0;
    private pendingRequests: Map<number, {
        resolve: (value: DaemonResponse) => void;
        reject: (reason: Error) => void;
    }> = new Map();
    private buffer = '';
    
    constructor(
        private daemonPath: string,
        private output: OrthosOutputChannel
    ) {}
    
    isRunning(): boolean {
        return this.process !== null && !this.process.killed;
    }
    
    async start(): Promise<void> {
        return new Promise((resolve, reject) => {
            this.output.log(`Starting daemon: ${this.daemonPath}`);
            
            this.process = spawn(this.daemonPath, [], {
                stdio: ['pipe', 'pipe', 'pipe']
            });
            
            this.process.stdout?.on('data', (data: Buffer) => {
                this.handleData(data.toString());
            });
            
            this.process.stderr?.on('data', (data: Buffer) => {
                this.output.log(`[daemon] ${data.toString().trim()}`);
            });
            
            this.process.on('error', (err) => {
                this.output.log(`Daemon error: ${err.message}`);
                reject(err);
            });
            
            this.process.on('close', (code) => {
                this.output.log(`Daemon exited with code ${code}`);
                this.process = null;
                // Reject all pending requests
                for (const [, pending] of this.pendingRequests) {
                    pending.reject(new Error('Daemon closed'));
                }
                this.pendingRequests.clear();
            });
            
            // Give it a moment to start
            setTimeout(() => resolve(), 100);
        });
    }
    
    private handleData(data: string): void {
        this.buffer += data;
        
        // Process complete lines
        const lines = this.buffer.split('\n');
        this.buffer = lines.pop() || '';
        
        for (const line of lines) {
            if (!line.trim()) continue;
            
            try {
                const response: DaemonResponse = JSON.parse(line);
                const id = response.id;
                
                if (id !== undefined && this.pendingRequests.has(id)) {
                    const pending = this.pendingRequests.get(id)!;
                    this.pendingRequests.delete(id);
                    pending.resolve(response);
                }
            } catch (err) {
                this.output.log(`Failed to parse daemon response: ${line}`);
            }
        }
    }
    
    private send(method: string, params?: Record<string, unknown>): Promise<DaemonResponse> {
        return new Promise((resolve, reject) => {
            if (!this.process || !this.process.stdin) {
                reject(new Error('Daemon not running'));
                return;
            }
            
            this.requestId++;
            const id = this.requestId;
            
            const request: Record<string, unknown> = { method, id };
            if (params) {
                request.params = params;
            }
            
            this.pendingRequests.set(id, { resolve, reject });
            
            const json = JSON.stringify(request);
            this.process.stdin.write(json + '\n');
            
            // Timeout after 30 seconds
            setTimeout(() => {
                if (this.pendingRequests.has(id)) {
                    this.pendingRequests.delete(id);
                    reject(new Error('Request timeout'));
                }
            }, 30000);
        });
    }
    
    async initialize(source: string): Promise<DaemonResponse> {
        return this.send('initialize', { source });
    }
    
    async checkpoint(): Promise<DaemonResponse> {
        return this.send('checkpoint');
    }
    
    async restore(): Promise<DaemonResponse> {
        return this.send('restore');
    }
    
    async constrain(laws: string[]): Promise<DaemonResponse> {
        return this.send('constrain', { laws });
    }
    
    async query(flux: string[]): Promise<DaemonResponse> {
        return this.send('query', { flux });
    }
    
    async shutdown(): Promise<void> {
        if (this.process) {
            try {
                await this.send('shutdown');
            } catch {
                // Ignore - daemon may have already closed
            }
            this.process.kill();
            this.process = null;
        }
    }
}
