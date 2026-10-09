import { en, type Key } from "./en";
import { pl, plData } from "./pl";

export type Lang = "en" | "pl";
/** Stored preference: a language or "auto" (follow the system) */
export type LangPref = Lang | "auto";

/** Languages offered in Preferences, named in their own language */
export const LANGUAGES: { code: Lang; name: string }[] = [
  { code: "en", name: "English" },
  { code: "pl", name: "Polski" },
];

const dicts: Record<Lang, Record<Key, string>> = { en, pl };
const data: Record<Lang, Record<string, string>> = { en: {}, pl: plData };

function systemLang(): Lang {
  return navigator.languages.some((l) => l.toLowerCase().startsWith("pl")) ? "pl" : "en";
}

export const i18n = $state<{ lang: Lang }>({ lang: systemLang() });

export function setLanguage(pref: string | undefined) {
  i18n.lang = pref === "en" || pref === "pl" ? pref : systemLang();
  document.documentElement.lang = i18n.lang;
}

function format(text: string, params?: Record<string, string | number>) {
  return params ? text.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m)) : text;
}

/** UI string in the current language. */
export function t(key: Key, params?: Record<string, string | number>): string {
  return format(dicts[i18n.lang][key] ?? en[key], params);
}

/** Device data from rivalcfg (labels, descriptions, choices, media keys); English text is the key. */
export function td(text: string): string {
  return data[i18n.lang][text] ?? text;
}

/** Display name of a profile: the built-in "Default" profile is translated. */
export function profileName(name: string): string {
  return name === "Default" ? t("defaultProfile") : name;
}

/**
 * Backend errors: user-facing ones arrive as "@code|arg" and are translated;
 * anything else (technical device errors) is shown as is.
 */
export function translateError(message: string): string {
  return message.replace(/@([a-z_]+)\|([\s\S]*)$/, (whole, code: string, arg: string) => {
    const key = `err_${code}` as Key;
    return key in en ? t(key, { arg }) : whole;
  });
}
