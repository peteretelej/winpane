// Ambient types for example scripts (avoids requiring @types/node install)
declare const process: {
  exit(code?: number): never;
  env: Record<string, string | undefined>;
  on(event: string, listener: (...args: any[]) => void): void;
};
declare function setTimeout(cb: (...args: any[]) => void, ms?: number, ...args: any[]): number;
declare function setInterval(cb: (...args: any[]) => void, ms?: number, ...args: any[]): number;
declare function clearInterval(id: number): void;
declare function clearTimeout(id: number): void;
