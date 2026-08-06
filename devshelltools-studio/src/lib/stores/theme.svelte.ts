/** Studio 主题：明 / 暗 / 彩 */

export type ThemeId = "light" | "dark" | "color";

const STORAGE_KEY = "dst-theme";

export const THEME_OPTIONS: { id: ThemeId; label: string }[] = [
  { id: "light", label: "明" },
  { id: "dark", label: "暗" },
  { id: "color", label: "彩" }
];

let current = $state<ThemeId>("dark");

function isThemeId(v: string | null | undefined): v is ThemeId {
  return v === "light" || v === "dark" || v === "color";
}

function applyDom(theme: ThemeId) {
  const root = document.documentElement;
  root.dataset.theme = theme;
  root.style.colorScheme = theme === "light" ? "light" : "dark";
}

/** 启动时调用（须在 mount App 之前）。 */
export function initTheme(): ThemeId {
  let theme: ThemeId = "dark";
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (isThemeId(saved)) theme = saved;
  } catch {
    /* ignore */
  }
  current = theme;
  applyDom(theme);
  return theme;
}

export function getTheme(): ThemeId {
  return current;
}

export function setTheme(theme: ThemeId) {
  current = theme;
  applyDom(theme);
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    /* ignore */
  }
}

/** 供模板订阅当前主题（runes）。 */
export function themeState() {
  return {
    get theme() {
      return current;
    }
  };
}
