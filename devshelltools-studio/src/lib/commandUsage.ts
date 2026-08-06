import type { PsFunction, PsParam } from "./api";

/** 生成友好用法：`gg [Count]` */
export function formatUsage(fn: PsFunction): string {
  const params = fn.parameters ?? [];
  if (params.length === 0) return fn.name;

  const parts = params.map((p) => formatParamToken(p));
  return `${fn.name} ${parts.join(" ")}`.trim();
}

function formatParamToken(p: PsParam): string {
  if (p.is_switch) {
    return p.mandatory ? `-${p.name}` : `[-${p.name}]`;
  }
  const token = p.name;
  if (p.mandatory && p.default_value == null) {
    return `<${token}>`;
  }
  return `[${token}]`;
}

/** 单行参数说明：`Count：提交历史数量，默认 20` */
export function formatParamLine(p: PsParam): string {
  const bits: string[] = [];
  const desc = (p.description || "").trim();
  if (desc) {
    bits.push(desc);
  } else if (p.is_switch) {
    bits.push("开关");
  } else {
    bits.push(p.type_name || "参数");
  }
  if (p.mandatory && p.default_value == null) {
    bits.push("必填");
  }
  if (p.default_value != null && p.default_value !== "") {
    bits.push(`默认 ${p.default_value}`);
  }
  return `${p.name}：${bits.join("，")}`;
}

/** 展开区用的示例行（多示例用 · 连接） */
export function formatExamples(fn: PsFunction): string {
  const ex = (
    fn.examples && fn.examples.length > 0
      ? fn.examples
      : fn.first_example
        ? [fn.first_example]
        : [fn.name]
  ).filter(Boolean);
  return ex.join(" · ");
}

/** 有默认可编辑的参数 */
export function editableDefaultParams(fn: PsFunction): PsParam[] {
  return (fn.parameters ?? []).filter(
    (p) => !p.is_switch && p.default_value != null && p.default_value !== ""
  );
}
