import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RawTrafficLog } from "../types";

export function useTraffic(isPollingActive: boolean) {
  const [trafficLogs, setTrafficLogs] = useState<RawTrafficLog[]>([]);
  const [selectedLog, setSelectedLog] = useState<RawTrafficLog | null>(null);

  const fetchTraffic = useCallback(async () => {
    try {
      const list = await invoke<RawTrafficLog[]>("get_raw_traffic");
      setTrafficLogs(list.reverse());
    } catch (e) {
      console.error("fetchTraffic error:", e);
    }
  }, []);

  useEffect(() => {
    fetchTraffic();
    if (!isPollingActive) return;

    const interval = setInterval(() => {
      fetchTraffic();
    }, 1500);

    return () => clearInterval(interval);
  }, [fetchTraffic, isPollingActive]);

  const clearLogs = useCallback(async () => {
    try {
      await invoke("clear_logs");
      setTrafficLogs([]);
      setSelectedLog(null);
    } catch (e) {
      console.error("clearLogs error:", e);
    }
  }, []);

  return {
    trafficLogs,
    selectedLog,
    setSelectedLog,
    fetchTraffic,
    clearLogs,
  };
}
