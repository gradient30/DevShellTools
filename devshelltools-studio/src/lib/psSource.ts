/**
 * 从分类 .ps1 全文中提取指定函数源码（含其前的注释帮助块）。
 */
export function extractFunctionSource(
  content: string,
  funcName: string
): string | null {
  if (!content || !funcName) return null;
  const escaped = funcName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const headerRe = new RegExp(`function\\s+${escaped}\\b`, "i");
  const start = content.search(headerRe);
  if (start < 0) return null;

  let from = start;
  const before = content.slice(0, start);
  const helpMatch = before.match(/(<#[\s\S]*?#>\s*)$/);
  if (helpMatch) {
    from = start - helpMatch[1].length;
  }

  const braceStart = content.indexOf("{", start);
  if (braceStart < 0) {
    return content.slice(from, Math.min(content.length, start + 240)).trim();
  }

  let depth = 0;
  for (let i = braceStart; i < content.length; i++) {
    const ch = content[i];
    if (ch === "{") depth += 1;
    else if (ch === "}") {
      depth -= 1;
      if (depth === 0) {
        return content.slice(from, i + 1).trim();
      }
    }
  }
  return content.slice(from).trim();
}

export interface AiCommandReviewContext {
  categoryTitle: string;
  categoryName: string;
  fileName: string;
  funcName: string;
  synopsis: string;
  example: string;
  siblingNames: string[];
  source: string | null;
}

/**
 * 构造「检查 / 修复 / 优化 / 扩展」导向的 AI 提问。
 */
export function buildCommandReviewPrompt(ctx: AiCommandReviewContext): string {
  const siblings = ctx.siblingNames.filter((n) => n !== ctx.funcName).join(", ") || "(无)";
  const sourceBlock = ctx.source
    ? `\`\`\`powershell\n${ctx.source}\n\`\`\``
    : "(未能从源文件提取到该函数完整源码，请根据名称与说明审阅。)";

  return [
    "你是 DevShellTools 命令审阅助手。请针对下列【现有】快捷命令做检查与优化，直接给出结论，不要寒暄。",
    "",
    `【分类】${ctx.categoryTitle}（${ctx.categoryName}）· 文件 ${ctx.fileName}`,
    `【目标命令】${ctx.funcName}`,
    `【说明】${ctx.synopsis || "(无)"}`,
    `【示例】${ctx.example || ctx.funcName}`,
    `【同分类其他命令】${siblings}`,
    "",
    "【当前源码】",
    sourceBlock,
    "",
    "请严格按下面三部分回复：",
    "1. 问题检查：若存在语法、安全边界、健壮性或可用性的问题，逐条说明，并给出修复后的完整函数（使用 powershell 代码块）。若无明显问题，明确写「未发现明显问题」。",
    "2. 优化建议：在不违反安全边界的前提下，提出能增强体验或能力的改进；若建议改代码，请给出完整函数代码块。",
    "3. 扩展建议：基于本分类，建议 1～3 个值得新增的常用相关命令（名称 + 一句话用途；如需示例实现可用代码块）。",
    "",
    "安全红线（必须遵守）：禁止 force-push / hard reset / 真实 git clean；禁止写 User 级环境变量；Stop-Process 须确认；禁止危险递归删除。",
  ].join("\n");
}
