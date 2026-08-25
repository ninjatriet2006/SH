export class HotkeyMap {
  private map: Record<string, () => void> = {};
  register(key: string, handler: () => void) { this.map[key] = handler; }
  getHandler(key: string) { return this.map[key]; }
}
