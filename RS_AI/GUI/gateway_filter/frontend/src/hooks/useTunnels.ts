import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { OutboundTunnel, TunnelEvent } from "../types";

export function useTunnels(onConfigRefresh: () => Promise<void>) {
  const [activeCliTunnelId, setActiveCliTunnelId] = useState<string | null>(null);
  const [tunnelEvents, setTunnelEvents] = useState<Record<string, TunnelEvent[]>>({});
  // tunnelId -> URL authorize đang chờ (phiên login nền)
  const [loginUrls, setLoginUrls] = useState<Record<string, string>>({});

  const fetchActiveCliTunnel = useCallback(async () => {
    try {
      const id = await invoke<string | null>("get_active_cli_tunnel");
      setActiveCliTunnelId(id);
    } catch (e) {
      console.error("fetchActiveCliTunnel error:", e);
    }
  }, []);

  const fetchTunnelEvents = useCallback(async () => {
    try {
      const map = await invoke<Record<string, TunnelEvent[]>>("get_tunnel_events");
      setTunnelEvents(map);
    } catch (e) {
      console.error("fetchTunnelEvents error:", e);
    }
  }, []);

  const fetchActiveLogins = useCallback(async () => {
    try {
      const map = await invoke<Record<string, string>>("get_active_logins");
      setLoginUrls(map);
    } catch (e) {
      console.error("fetchActiveLogins error:", e);
    }
  }, []);

  // Lấy fetchConfig mới nhất mà không tạo vòng deps (onConfigRefresh đã ổn định từ useConfig)
  const fetchConfig_safe = useCallback(() => {
    onConfigRefresh().catch((e) => console.error(e));
  }, [onConfigRefresh]);

  useEffect(() => {
    fetchActiveCliTunnel();
    fetchTunnelEvents();
    fetchActiveLogins();
    const interval = setInterval(fetchActiveCliTunnel, 1500);
    const unlisten = listen("tunnel-status-changed", () => {
      fetchTunnelEvents();
    });
    const unlistenLoginUrl = listen<{ tunnel_id: string; url: string }>(
      "login-url-ready",
      (event) => {
        setLoginUrls((prev) => ({ ...prev, [event.payload.tunnel_id]: event.payload.url }));
        fetchTunnelEvents();
      }
    );
    const unlistenLoginDone = listen<{ tunnel_id: string; success: boolean }>(
      "login-finished",
      (event) => {
        setLoginUrls((prev) => {
          const next = { ...prev };
          delete next[event.payload.tunnel_id];
          return next;
        });
        fetchTunnelEvents();
        fetchConfig_safe();
      }
    );
    return () => {
      clearInterval(interval);
      unlisten.then((u) => u());
      unlistenLoginUrl.then((u) => u());
      unlistenLoginDone.then((u) => u());
    };
  }, [fetchActiveCliTunnel, fetchTunnelEvents, fetchActiveLogins, fetchConfig_safe]);

  const testTunnel = useCallback(async (tunnel: OutboundTunnel) => {
    try {
      await invoke("test_single_tunnel", { tunnel });
      await onConfigRefresh();
      await fetchTunnelEvents();
    } catch (err) {
      console.error("testTunnel error:", err);
    }
  }, [onConfigRefresh, fetchTunnelEvents]);

  const toggleTunnel = useCallback(async (tunnelId: string, enabled: boolean) => {
    try {
      await invoke("toggle_tunnel", { tunnelId, enabled });
      await onConfigRefresh();
      await fetchActiveCliTunnel();
      await fetchTunnelEvents();
    } catch (e) {
      console.error("toggleTunnel error:", e);
      await onConfigRefresh();
      await fetchActiveCliTunnel();
      await fetchTunnelEvents();
      window.alert(String(e));
    }
  }, [onConfigRefresh, fetchActiveCliTunnel, fetchTunnelEvents]);

  const forceStopTunnel = useCallback(async (tunnelId: string) => {
    try {
      await invoke("force_stop_tunnel", { tunnelId });
      await onConfigRefresh();
      await fetchActiveCliTunnel();
      await fetchTunnelEvents();
    } catch (e) {
      console.error("forceStopTunnel error:", e);
      window.alert(String(e));
    }
  }, [onConfigRefresh, fetchActiveCliTunnel, fetchTunnelEvents]);

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

  const loginTunnel = useCallback(async (tunnelId: string) => {
    try {
      const msg = await invoke<string>("start_cli_login", { tunnelId });
      await fetchTunnelEvents();
      await fetchActiveLogins();
      return msg;
    } catch (e) {
      console.error("loginTunnel error:", e);
      await fetchTunnelEvents();
      window.alert(String(e));
      throw e;
    }
  }, [fetchTunnelEvents, fetchActiveLogins]);

  const cancelLogin = useCallback(async (tunnelId: string) => {
    try {
      await invoke("cancel_cli_login", { tunnelId });
      await fetchTunnelEvents();
      await fetchActiveLogins();
    } catch (e) {
      console.error("cancelLogin error:", e);
    }
  }, [fetchTunnelEvents, fetchActiveLogins]);

  const openLoginUrl = useCallback(async (url: string) => {
    try {
      await invoke("open_url", { url });
    } catch (e) {
      console.error("openLoginUrl error:", e);
      window.alert(String(e));
    }
  }, []);

  return {
    activeCliTunnelId,
    tunnelEvents,
    loginUrls,
    fetchTunnelEvents,
    testTunnel,
    toggleTunnel,
    forceStopTunnel,
    addOrUpdateTunnel,
    deleteTunnel,
    loginTunnel,
    cancelLogin,
    openLoginUrl,
    fetchActiveCliTunnel,
  };
}
