const TOKEN_PREFIX = '\u0000';

export function renderMarkdown(markdown: string) {
  const lines = markdown.replace(/\r\n?/g, '\n').split('\n');
  const output: string[] = [];
  let paragraph: string[] = [];
  let list: 'ul' | 'ol' | null = null;
  let code: string[] | null = null;

  const flushParagraph = () => {
    if (paragraph.length === 0) return;
    output.push(`<p>${paragraph.map(renderInline).join('<br>')}</p>`);
    paragraph = [];
  };
  const closeList = () => {
    if (!list) return;
    output.push(`</${list}>`);
    list = null;
  };

  for (const line of lines) {
    if (line.trimStart().startsWith('```')) {
      flushParagraph();
      closeList();
      if (code) {
        output.push(`<pre><code>${escapeHtml(code.join('\n'))}</code></pre>`);
        code = null;
      } else {
        code = [];
      }
      continue;
    }
    if (code) {
      code.push(line);
      continue;
    }

    const heading = line.match(/^(#{1,3})\s+(.+)$/);
    if (heading) {
      flushParagraph();
      closeList();
      const level = heading[1].length;
      output.push(`<h${level}>${renderInline(heading[2])}</h${level}>`);
      continue;
    }

    const quote = line.match(/^>\s?(.*)$/);
    if (quote) {
      flushParagraph();
      closeList();
      output.push(`<blockquote>${renderInline(quote[1])}</blockquote>`);
      continue;
    }

    const unordered = line.match(/^\s*[-*+]\s+(.+)$/);
    const ordered = line.match(/^\s*\d+[.)]\s+(.+)$/);
    if (unordered || ordered) {
      flushParagraph();
      const nextList = unordered ? 'ul' : 'ol';
      if (list !== nextList) {
        closeList();
        output.push(`<${nextList}>`);
        list = nextList;
      }
      output.push(`<li>${renderInline((unordered || ordered)![1])}</li>`);
      continue;
    }

    if (!line.trim()) {
      flushParagraph();
      closeList();
      continue;
    }

    closeList();
    paragraph.push(line);
  }

  if (code) output.push(`<pre><code>${escapeHtml(code.join('\n'))}</code></pre>`);
  flushParagraph();
  closeList();
  return output.join('');
}

function renderInline(value: string) {
  const tokens: string[] = [];
  let rendered = escapeHtml(value).replace(/`([^`]+)`/g, (_match, code: string) => {
    const token = `${TOKEN_PREFIX}${tokens.length}${TOKEN_PREFIX}`;
    tokens.push(`<code>${code}</code>`);
    return token;
  });

  rendered = rendered.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_match, label: string, href: string) => {
    const safeHref = safeUrl(href);
    if (!safeHref) return label;
    const token = `${TOKEN_PREFIX}${tokens.length}${TOKEN_PREFIX}`;
    tokens.push(`<a href="${escapeHtml(safeHref)}" target="_blank" rel="noreferrer">${label}</a>`);
    return token;
  });
  rendered = rendered
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/__(.+?)__/g, '<strong>$1</strong>')
    .replace(/~~(.+?)~~/g, '<del>$1</del>')
    .replace(/(^|[^*])\*([^*]+)\*(?!\*)/g, '$1<em>$2</em>')
    .replace(/(^|[^_])_([^_]+)_(?!_)/g, '$1<em>$2</em>');

  return rendered.replace(/\u0000(\d+)\u0000/g, (_match, index: string) => tokens[Number(index)] ?? '');
}

function safeUrl(value: string) {
  try {
    const url = new URL(value, 'https://github.com');
    return ['http:', 'https:'].includes(url.protocol) ? url.href : null;
  } catch {
    return null;
  }
}

function escapeHtml(value: string) {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
