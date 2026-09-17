import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { GatewayConfig, RouteRule } from "../types";

export function useConfig() {
  const [config, setConfig] = useState<GatewayConfig | null>(null);
  const [bindError, setBindError] = useState<{ port: number; error: string } | null>(null);

  const fetchConfig = useCallback(async () => {
    try {
      const cfg = await invoke<GatewayConfig>("get_config");
      setConfig(cfg);
    } catch (e) {
      console.error("fetchConfig error:", e);
    }
  }, []);

  useEffect(() => {
    fetchConfig();

    const unlistenBind = listen<{ port: number; error: string }>(
      "endpoint-bind-error",
      (event) => {
        setBindError(event.payload);
        fetchConfig();
      }
    );

    const unlistenKeys = listen("route-keys-updated", () => {
      fetchConfig();
    });

    const unlistenTunnels = listen("tunnel-status-changed", () => {
      fetchConfig();
    });

    return () => {
      unlistenBind.then((u) => u());
      unlistenKeys.then((u) => u());
      unlistenTunnels.then((u) => u());
    };
  }, [fetchConfig]);

  const saveConfig = useCallback(async (newConfig: GatewayConfig) => {
    try {
      await invoke("save_config", { newConfig });
      setConfig(newConfig);
    } catch (e) {
      console.error("saveConfig error:", e);
      throw e;
    }
  }, []);

  const addOrUpdateRoute = useCallback(async (route: RouteRule) => {
    try {
      await invoke("add_or_update_route", { route });
      await fetchConfig();
    } catch (e) {
      console.error("addOrUpdateRoute error:", e);
      throw e;
    }
  }, [fetchConfig]);

  const deleteRoute = useCallback(async (routeId: string) => {
    try {
      await invoke("delete_route", { routeId });
      await fetchConfig();
    } catch (e) {
      console.error("deleteRoute error:", e);
      throw e;
    }
  }, [fetchConfig]);

  const toggleRoute = useCallback(async (routeId: string, enabled: boolean) => {
    try {
      setBindError(null);
      await invoke("toggle_route", { routeId, enabled });
      await fetchConfig();
    } catch (e) {
      console.error("toggleRoute error:", e);
      window.alert(String(e));
    }
  }, [fetchConfig]);

  const advanceKey = useCallback(async (routeId: string) => {
    try {
      await invoke("advance_endpoint_key", { routeId });
      await fetchConfig();
    } catch (e) {
      console.error("advanceKey error:", e);
    }
  }, [fetchConfig]);

  return {
    config,
    bindError,
    setBindError,
    fetchConfig,
    saveConfig,
    addOrUpdateRoute,
    deleteRoute,
    toggleRoute,
    advanceKey,
  };
}
