import { useSettingsStore } from "../store/useSettingsStore";
import { translate } from "./resources";

export function useTranslation() {
  const messages = useSettingsStore(state => state.messages);

  const t = (key: string): string => translate(messages, key);

  return { t };
}
