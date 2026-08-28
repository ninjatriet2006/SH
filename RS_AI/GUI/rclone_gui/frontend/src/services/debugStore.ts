export interface LogEntry {
  timestamp: string;
  type: 'INFO' | 'API' | 'TRANSFER' | 'SYS';
  action: string;
  detail: string;
}

class DebugStore {
  private logs: LogEntry[] = [];
  private maxLogs = 500;
  private listeners: ((logs: LogEntry[]) => void)[] = [];

  public log(type: LogEntry['type'], action: string, detail: any) {
    const detailStr = typeof detail === 'string' ? detail : JSON.stringify(detail);
    const entry: LogEntry = {
      timestamp: new Date().toLocaleTimeString('vi-VN', { hour12: false }) + '.' + new Date().getMilliseconds().toString().padStart(3, '0'),
      type,
      action,
      detail: detailStr
    };
    
    this.logs.push(entry);
    if (this.logs.length > this.maxLogs) {
      this.logs.shift();
    }
    
    this.notify();
  }

  public getLogs(): LogEntry[] {
    return this.logs;
  }

  public clear() {
    this.logs = [];
    this.notify();
  }

  public subscribe(callback: (logs: LogEntry[]) => void) {
    this.listeners.push(callback);
  }

  public unsubscribe(callback: (logs: LogEntry[]) => void) {
    this.listeners = this.listeners.filter(cb => cb !== callback);
  }

  private notify() {
    for (const listener of this.listeners) {
      listener(this.logs);
    }
  }
}

export const debugStore = new DebugStore();
