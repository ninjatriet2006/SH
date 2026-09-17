import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OutboundTunnel } from "../types";

export function useTunnels(onConfigRefresh: () => Promise<void>) {
  const [activeCliTunnelId, setActiveCliTunnelId] = useState<string | null>(null);

  const fetchActiveCliTunnel = useCallback(async () => {
    try {
      const id = await invoke<string | null>("get_active_cli_tunnel");
      setActiveCliTunnelId(id);
    } catch (e) {
      console.error("fetchActiveCliTunnel error:", e);
    }
  }, []);

  useEffect(() => {
    fetchActiveCliTunnel();
    const interval = setInterval(fetchActiveCliTunnel, 1500);
    return () => clearInterval(interval);
  }, [fetchActiveCliTunnel]);

  const testTunnel = useCallback(async (tunnel: OutboundTunnel) => {
    try {
      await invoke("test_single_tunnel", { tunnel });
      await onConfigRefresh();
    } catch (err) {
      console.error("testTunnel error:", err);
    }
  }, [onConfigRefresh]);

  const toggleTunnel = useCallback(async (tunnelId: string, enabled: boolean) => {
    try {
      await invoke("toggle_tunnel", { tunnelId, enabled });
      await onConfigRefresh();
      await fetchActiveCliTunnel();
    } catch (e) {
      console.error("toggleTunnel error:", e);
      await onConfigRefresh();
      await fetchActiveCliTunnel();
      window.alert(String(e));
    }
  }, [onConfigRefresh, fetchActiveCliTunnel]);

  const forceStopTunnel = useCallback(async (tunnelId: string) => {
    try {
      await invoke("force_stop_tunnel", { tunnelId });
      await onConfigRefresh();
      await fetchActiveCliTunnel();
    } catch (e) {
      console.error("forceStopTunnel error:", e);
      window.alert(String(e));
    }
  }, [onConfigRefresh, fetchActiveCliTunnel]);

  const addOrUpdateTunnel = useCallback(async (tunnel: OutboundTunnel) => {
    try {
      await invoke("add_or_update_tunnel", { tunnel });
      await onConfigRefresh();
    } catch (e) {
      console.error("addOrUpdateTunnel error:", e);
      throw e;
    }
  }, [onConfigRefresh]);

  const deleteTunnel = useCallback(async (tunnelId: string) => {
    try {
      await invoke("delete_tunnel", { tunnelId });
      await onConfigRefresh();
    } catch (e) {
      console.error("deleteTunnel error:", e);
      throw e;
    }
  }, [onConfigRefresh]);

  return {
    activeCliTunnelId,
    testTunnel,
    toggleTunnel,
    forceStopTunnel,
    addOrUpdateTunnel,
    deleteTunnel,
    fetchActiveCliTunnel,
  };
}
