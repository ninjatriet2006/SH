import React, { createContext, useContext, useState, useEffect } from 'react';
import { accountHubApi } from '../../bridge/api';

interface LangContextType {
  lang: string;
  setLang: (lang: string) => void;
  t: (key: string) => string;
}

const LangContext = createContext<LangContextType>({
  lang: 'vi',
  setLang: () => {},
  t: (key: string) => key,
});

export const LangProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [lang, setLangState] = useState<string>('vi');
  const [translations, setTranslations] = useState<any>({});

  useEffect(() => {
    const loadTranslations = async () => {
      try {
        const langs = await accountHubApi.getLanguages();
        const map: any = {};
        for (const l of langs) {
          if (l.langs && l.langs[0]) {
            map[l.langs[0].id] = l;
          }
        }
        setTranslations(map);
      } catch (e) {
        console.error('Failed to load languages', e);
      }
    };
    loadTranslations();
  }, []);

  const setLang = (newLang: string) => {
    setLangState(newLang);
  };

  const t = (path: string): string => {
    const currentMap = translations[lang] || translations['vi'] || {};
    const parts = path.split('.');
    let val = currentMap;
    for (const part of parts) {
      if (val && typeof val === 'object' && part in val) {
        val = val[part];
      } else {
        return path;
      }
    }
    return typeof val === 'string' ? val : path;
  };

  return (
    <LangContext.Provider value={{ lang, setLang, t }}>
      {children}
    </LangContext.Provider>
  );
};

export const useTranslation = () => useContext(LangContext);
