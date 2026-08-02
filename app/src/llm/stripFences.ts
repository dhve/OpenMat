// Defensive fence stripping. The system prompt instructs the model to
// return only code with no markdown fences, but models do not always
// follow that instruction exactly, so this runs on every response before
// it reaches the preview.
export function stripCodeFences(raw: string): string {
  let text = raw.trim();
  if (text === "") return text;

  // A complete ```lang\n...\n``` (or plain ```\n...\n```) fence, language
  // tag optional. The newline after the opening fence is required here so
  // a single-line reply like ```Plot[x]``` (no language, no newline) isn't
  // misread as an empty-code block with "Plot[x]" as the language tag;
  // that case falls through to the partial handling below instead.
  const fullFence = /^```([a-zA-Z0-9_-]*)\r?\n([\s\S]*?)\r?\n?```$/.exec(text);
  if (fullFence) {
    return fullFence[2].trim();
  }

  // Truncated or malformed response carrying only one side of the fence.
  if (text.startsWith("```")) {
    const firstNewline = text.indexOf("\n");
    text = firstNewline === -1 ? text.slice(3) : text.slice(firstNewline + 1);
  }
  if (text.endsWith("```")) {
    text = text.slice(0, -3);
  }
  text = text.trim();

  // A single-line reply wrapped in one pair of backticks, e.g. `Sin[x]`.
  if (text.length >= 2 && text.startsWith("`") && text.endsWith("`")) {
    text = text.slice(1, -1).trim();
  }

  return text;
}
