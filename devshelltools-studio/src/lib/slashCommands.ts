/** AI 聊天区斜杠命令元数据（与 ChatPanel.handleSessionCommand 保持同步）。 */

export interface SlashCommand {
  /** 完整命令，含前导 `/` */
  name: string;
  /** 同行简介 */
  description: string;
  /** 仅 `/resume` 选号模式显示 */
  resumeOnly?: boolean;
  /** 选号模式下隐藏 */
  hideInResume?: boolean;
}

export const SLASH_COMMANDS: SlashCommand[] = [
  {
    name: "/resume",
    description: "列出历史会话，按编号恢复"
  },
  {
    name: "/sessions",
    description: "同 /resume，列出历史会话"
  },
  {
    name: "/new",
    description: "新建空会话（先保存当前）",
    hideInResume: true
  },
  {
    name: "/danger",
    description: "本会话开启危险模式（放宽红线）",
    hideInResume: true
  },
  {
    name: "/safe",
    description: "关闭危险模式，恢复默认红线",
    hideInResume: true
  },
  {
    name: "/cancel",
    description: "取消 /resume 选号",
    resumeOnly: true
  }
];

/**
 * 从输入解析 slash 前缀（不含 `/`）。
 * 非 slash、含空白/换行、选号下纯数字 → null（不展示面板）。
 */
export function parseSlashQuery(
  raw: string,
  resumeMode: boolean
): string | null {
  if (!raw.startsWith("/")) return null;
  if (raw.includes("\n") || raw.includes("\r")) return null;
  if (/\s/.test(raw)) return null;
  const trimmed = raw.trim();
  if (resumeMode && /^\d+$/.test(trimmed)) return null;
  return raw.slice(1).toLowerCase();
}

/** 按前缀过滤；顺序与 SLASH_COMMANDS 一致。 */
export function filterSlashCommands(
  prefix: string,
  resumeMode: boolean
): SlashCommand[] {
  const p = prefix.toLowerCase().replace(/^\//, "");
  return SLASH_COMMANDS.filter((c) => {
    if (resumeMode && c.hideInResume) return false;
    if (!resumeMode && c.resumeOnly) return false;
    const key = c.name.slice(1).toLowerCase();
    return key.startsWith(p);
  });
}
