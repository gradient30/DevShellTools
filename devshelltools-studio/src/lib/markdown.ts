/**
 * 轻量 Markdown → 安全 HTML（仅常用语法，无依赖）。
 * 先转义再标记，避免 AI 回复注入 HTML。
 */

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function inlineFormat(escaped: string): string {
  let s = escaped;
  // `code`
  s = s.replace(/`([^`]+)`/g, "<code>$1</code>");
  // **bold** / __bold__
  s = s.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/__([^_]+)__/g, "<strong>$1</strong>");
  // *italic*（避免与列表冲突：仅匹配非行首的成对 *）
  s = s.replace(/(^|[\s（(])\*([^*\n]+)\*(?=[\s）。)、,，.!？?]|$)/g, "$1<em>$2</em>");
  return s;
}

/**
 * 将 Markdown 纯文本转为可读 HTML（段落/标题/列表/粗体/行内代码/分隔线）。
 */
export function markdownToHtml(src: string): string {
  const text = (src ?? "").replace(/\r\n/g, "\n").trim();
  if (!text) return "";

  const lines = text.split("\n");
  const out: string[] = [];
  let i = 0;
  let para: string[] = [];

  const flushPara = () => {
    if (para.length === 0) return;
    const body = inlineFormat(escapeHtml(para.join("\n"))).replace(/\n/g, "<br>");
    out.push(`<p>${body}</p>`);
    para = [];
  };

  while (i < lines.length) {
    const line = lines[i] ?? "";
    const trimmed = line.trim();

    // 空行 → 段间隔
    if (trimmed === "") {
      flushPara();
      i += 1;
      continue;
    }

    // 分隔线
    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      flushPara();
      out.push("<hr>");
      i += 1;
      continue;
    }

    // 标题
    const hm = /^(#{1,4})\s+(.+)$/.exec(trimmed);
    if (hm) {
      flushPara();
      const level = Math.min(hm[1].length + 1, 4); // h2–h5，避免过大
      out.push(`<h${level}>${inlineFormat(escapeHtml(hm[2]))}</h${level}>`);
      i += 1;
      continue;
    }

    // 无序/有序列表
    const ul = /^[-*+]\s+(.+)$/.exec(trimmed);
    const ol = /^(\d+)[.)]\s+(.+)$/.exec(trimmed);
    if (ul || ol) {
      flushPara();
      const ordered = !!ol;
      const tag = ordered ? "ol" : "ul";
      const items: string[] = [];
      while (i < lines.length) {
        const t = (lines[i] ?? "").trim();
        const u = /^[-*+]\s+(.+)$/.exec(t);
        const o = /^(\d+)[.)]\s+(.+)$/.exec(t);
        if (ordered && o) {
          items.push(`<li>${inlineFormat(escapeHtml(o[2]))}</li>`);
          i += 1;
        } else if (!ordered && u) {
          items.push(`<li>${inlineFormat(escapeHtml(u[1]))}</li>`);
          i += 1;
        } else {
          break;
        }
      }
      out.push(`<${tag}>${items.join("")}</${tag}>`);
      continue;
    }

    // 引用
    if (trimmed.startsWith("> ")) {
      flushPara();
      const quotes: string[] = [];
      while (i < lines.length) {
        const t = (lines[i] ?? "").trim();
        if (!t.startsWith(">")) break;
        quotes.push(t.replace(/^>\s?/, ""));
        i += 1;
      }
      out.push(
        `<blockquote>${inlineFormat(escapeHtml(quotes.join("\n"))).replace(/\n/g, "<br>")}</blockquote>`
      );
      continue;
    }

    para.push(line);
    i += 1;
  }
  flushPara();
  return out.join("");
}
