import { invokeIpc } from './ipc';
import type { GuiSettings } from './types';

export async function getGuiSettings(): Promise<GuiSettings> {
  return invokeIpc<GuiSettings>('get_gui_settings');
}
export async function saveGuiSettings(settings: GuiSettings): Promise<void> {
  await invokeIpc<Record<string, never>, GuiSettings>('save_gui_settings', settings);
}
