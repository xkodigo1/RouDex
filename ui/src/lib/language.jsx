import { invoke } from "@tauri-apps/api/core";
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { setLanguage, systemLanguage, t } from "./i18n.js";

const LanguageContext = createContext(null);

/**
 * The interface language: `language` in router.toml, or on a first start the
 * system language, which is then stored there so the Desktop hook follows it.
 * Components re-render through `useI18n()` when it changes.
 */
export function LanguageProvider({ children }) {
  const [language, setCurrent] = useState(() => {
    const initial = systemLanguage();
    setLanguage(initial);
    return initial;
  });

  const apply = useCallback((next) => {
    setLanguage(next);
    setCurrent(next);
  }, []);

  useEffect(() => {
    invoke("router_config").then((config) => {
      if (config?.language) {
        apply(config.language);
      } else {
        invoke("settings_set_language", { language: systemLanguage() }).catch(() => {});
      }
    }).catch(() => {});
  }, [apply]);

  const choose = useCallback(async (next) => {
    await invoke("settings_set_language", { language: next });
    apply(next);
  }, [apply]);

  const value = useMemo(() => ({ language, choose, t }), [language, choose]);
  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

/** `{ language, choose, t }`; using it re-renders on a language change. */
export function useI18n() {
  return useContext(LanguageContext);
}
