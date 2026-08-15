import { Terminal } from '@xterm/xterm';

export interface DetectedTarget {
  type: 'url' | 'file' | 'directory';
  rawText: string;
  cleanedPath?: string;
  line?: number;
  col?: number;
  windowsPath?: string;
  gitBashPath?: string;
  wslPath?: string;
  startCol?: number;
  bufferY?: number;
  length?: number;
}

/**
 * Clean path of trailing punctuation, terminal prompt prefixes, and line/col suffixes like :123:45 or (123,45).
 */
export function parsePathSuffix(inputPath: string): { cleanPath: string; line?: number; col?: number } {
  let text = inputPath.trim();

  // Strip leading quotes, brackets, or prompt decorations
  text = text.replace(/^["'(<\[]+/, '');

  // Strip common shell prompt prefixes (e.g. user@host:~/path, (venv) user@host:path, PS C:\path>)
  text = text.replace(/^[a-zA-Z0-9_.-]+@[a-zA-Z0-9_.-]+:([/~.]|([a-zA-Z]:))/, '$1');
  text = text.replace(/^\[[a-zA-Z0-9_.-]+@[a-zA-Z0-9_.-]+\s+([^\]]+)\]$/, '$1');

  // Strip trailing shell prompt symbols (e.g. $, #, >, %)
  text = text.replace(/[$#>%]+$/, '');

  let line: number | undefined;
  let col: number | undefined;

  // Pattern: path:line:col or path:line
  const colonMatch = text.match(/^(.*?):(\d+)(?::(\d+))?([)"'\s,;:]*)$/);
  if (colonMatch) {
    text = colonMatch[1];
    line = parseInt(colonMatch[2], 10);
    if (colonMatch[3]) {
      col = parseInt(colonMatch[3], 10);
    }
  } else {
    // Pattern: path(line,col) or path(line)
    const parenMatch = text.match(/^(.*?)\((\d+)(?:,\s*(\d+))?\)([)"'\s,;:]*)$/);
    if (parenMatch) {
      text = parenMatch[1];
      line = parseInt(parenMatch[2], 10);
      if (parenMatch[3]) {
        col = parseInt(parenMatch[3], 10);
      }
    }
  }

  // Strip trailing quotes, brackets, or punctuation at end of line/prose
  text = text.replace(/["')>\]]+$/g, '');
  text = text.replace(/[.,;:!?]+$/, '');

  return { cleanPath: text, line, col };
}

/**
 * Check if string is a URL.
 */
export function detectUrl(text: string): string | null {
  const trimmed = text.trim().replace(/^["'(<]+|["')>]+$/g, '').replace(/[.,;:!?]+$/, '');

  // Standard URL with protocol
  if (/^(https?|ftp|file|ws|wss):\/\/[^\s"']+$/i.test(trimmed)) {
    return trimmed;
  }

  // Domain / Localhost URL
  if (/^(www\.[a-z0-9\-]+\.[a-z]{2,}|localhost:\d+|127\.0\.0\.1:\d+)(\/[^\s"']*)?$/i.test(trimmed)) {
    return `http://${trimmed}`;
  }

  return null;
}

/**
 * Convert Git Bash (/c/path) or WSL (/mnt/c/path) or Windows (C:\path) to Windows path format.
 */
export function toWindowsPath(path: string): string {
  let p = path.trim().replace(/\//g, '\\');

  // WSL format: \mnt\c\Users\... -> C:\Users\...
  const wslMatch = p.match(/^\\mnt\\([a-zA-Z])\\(.*)$/);
  if (wslMatch) {
    return `${wslMatch[1].toUpperCase()}:\\${wslMatch[2]}`;
  }

  // Git Bash format: \c\Users\... -> C:\Users\...
  const gitBashMatch = p.match(/^\\([a-zA-Z])\\(.*)$/);
  if (gitBashMatch) {
    return `${gitBashMatch[1].toUpperCase()}:\\${gitBashMatch[2]}`;
  }

  // Already Windows path with drive letter
  const winMatch = p.match(/^([a-zA-Z]):\\(.*)$/);
  if (winMatch) {
    return `${winMatch[1].toUpperCase()}:\\${winMatch[2]}`;
  }

  return p;
}

/**
 * Convert Windows (C:\path) or WSL (/mnt/c/path) to Git Bash format (/c/path).
 */
export function toGitBashPath(path: string): string {
  const winPath = toWindowsPath(path);
  const match = winPath.match(/^([a-zA-Z]):\\(.*)$/);
  if (match) {
    const drive = match[1].toLowerCase();
    const rest = match[2].replace(/\\/g, '/');
    return `/${drive}/${rest}`;
  }
  return path.replace(/\\/g, '/');
}

/**
 * Convert Windows (C:\path) or Git Bash (/c/path) to WSL format (/mnt/c/path).
 */
export function toWslPath(path: string): string {
  const winPath = toWindowsPath(path);
  const match = winPath.match(/^([a-zA-Z]):\\(.*)$/);
  if (match) {
    const drive = match[1].toLowerCase();
    const rest = match[2].replace(/\\/g, '/');
    return `/mnt/${drive}/${rest}`;
  }
  return path.replace(/\\/g, '/');
}

/**
 * Format a directory path appropriately for a specific shell profile.
 */
export function formatCwdForProfile(path: string, profileId: string): string {
  switch (profileId.toLowerCase()) {
    case 'wsl':
      return toWslPath(path);
    case 'git-bash':
    case 'gitbash':
    case 'bash':
      return toGitBashPath(path);
    case 'powershell':
    case 'cmd':
    default:
      return toWindowsPath(path);
  }
}

/**
 * Check if path string looks like a directory rather than a file with extension.
 */
export function isDirectoryTarget(cleanPath: string): boolean {
  if (!cleanPath) return false;
  if (cleanPath.endsWith('/') || cleanPath.endsWith('\\')) return true;
  const lastSegment = cleanPath.split(/[/\\]/).pop() || '';
  const hasExtension = /\.[a-zA-Z0-9_-]{1,8}$/.test(lastSegment);
  return !hasExtension;
}

/**
 * Check if text looks like a valid file or directory path (Windows, POSIX, WSL, Git Bash, Relative).
 */
export function detectFilePath(text: string): { cleanPath: string; isDir: boolean; line?: number; col?: number; windowsPath: string; gitBashPath: string; wslPath: string } | null {
  const { cleanPath, line, col } = parsePathSuffix(text);

  if (!cleanPath || cleanPath.length < 2) return null;

  // Check against path patterns
  const isWinAbs = /^[a-zA-Z]:[/\\]/i.test(cleanPath) || /^\\\\/i.test(cleanPath);
  const isGitBashAbs = /^\/[a-zA-Z]\//i.test(cleanPath);
  const isWslAbs = /^\/mnt\/[a-zA-Z]\//i.test(cleanPath);
  const isTildePath = /^~\//i.test(cleanPath) || cleanPath === '~';
  const isPosixAbs = /^\/[a-zA-Z0-9_\-.]+(\/[^\s"'<>*?|]+)*/i.test(cleanPath);
  const isRelative = /^(\.\.?\/|\.\.\\|[a-zA-Z0-9_\-.]+[/\\][a-zA-Z0-9_\-.])/i.test(cleanPath);

  if (isWinAbs || isGitBashAbs || isWslAbs || isTildePath || isPosixAbs || isRelative) {
    const windowsPath = toWindowsPath(cleanPath);
    const gitBashPath = toGitBashPath(cleanPath);
    const wslPath = toWslPath(cleanPath);
    const isDir = isDirectoryTarget(cleanPath);
    return { cleanPath, isDir, line, col, windowsPath, gitBashPath, wslPath };
  }

  return null;
}

/**
 * Extracts a token/word around a column index from a line of text, returning token and start column.
 */
export function extractTokenAtColumn(text: string, colIndex: number): { token: string; startCol: number } {
  if (!text || colIndex < 0 || colIndex >= text.length) {
    if (text.length > 0) {
      colIndex = Math.min(colIndex, text.length - 1);
    } else {
      return { token: '', startCol: 0 };
    }
  }

  const isDelim = (char: string) => /[\s"'<>`\t]/.test(char);

  let start = colIndex;
  while (start > 0 && !isDelim(text[start - 1])) {
    start--;
  }

  let end = colIndex;
  while (end < text.length && !isDelim(text[end])) {
    end++;
  }

  return { token: text.substring(start, end), startCol: start };
}

/**
 * Intelligently detect URL or File Path under mouse cursor or selection in xterm.
 */
export function detectCursorTarget(term: Terminal, event: MouseEvent): DetectedTarget | null {
  // 1. Prioritize active text selection if present
  if (term.hasSelection()) {
    const selection = term.getSelection().trim();
    if (selection) {
      const url = detectUrl(selection);
      if (url) {
        return { type: 'url', rawText: selection };
      }
      const pathInfo = detectFilePath(selection);
      if (pathInfo) {
        return {
          type: pathInfo.isDir ? 'directory' : 'file',
          rawText: selection,
          cleanedPath: pathInfo.cleanPath,
          line: pathInfo.line,
          col: pathInfo.col,
          windowsPath: pathInfo.windowsPath,
          gitBashPath: pathInfo.gitBashPath,
          wslPath: pathInfo.wslPath,
        };
      }
    }
  }

  // 2. Locate character token under mouse cursor position
  if (!term.element) return null;

  const rect = term.element.getBoundingClientRect();
  const mouseX = event.clientX - rect.left;
  const mouseY = event.clientY - rect.top;

  const core = (term as any)._core;
  let cellWidth = 8.42;
  let cellHeight = 17.0;

  if (core && core._renderService && core._renderService.dimensions) {
    const cssCell = core._renderService.dimensions.css.cell;
    if (cssCell && cssCell.width > 0 && cssCell.height > 0) {
      cellWidth = cssCell.width;
      cellHeight = cssCell.height;
    }
  }

  const padding = 8;
  const col = Math.floor((mouseX - padding) / cellWidth);
  const viewportRow = Math.floor((mouseY - padding) / cellHeight);

  if (viewportRow < 0 || viewportRow >= term.rows || col < 0 || col >= term.cols) {
    return null;
  }

  const bufferY = term.buffer.active.viewportY + viewportRow;
  const line = term.buffer.active.getLine(bufferY);
  if (!line) return null;

  let startY = bufferY;
  while (startY > 0) {
    const prevLine = term.buffer.active.getLine(startY);
    if (prevLine && prevLine.isWrapped) {
      startY--;
    } else {
      break;
    }
  }

  let endY = bufferY;
  while (endY < term.buffer.active.length - 1) {
    const nextLine = term.buffer.active.getLine(endY + 1);
    if (nextLine && nextLine.isWrapped) {
      endY++;
    } else {
      break;
    }
  }

  let fullLineText = '';
  let cursorCharIndex = 0;

  for (let y = startY; y <= endY; y++) {
    const l = term.buffer.active.getLine(y);
    if (!l) continue;
    const lText = l.translateToString(true);
    if (y < bufferY) {
      cursorCharIndex += lText.length;
    } else if (y === bufferY) {
      cursorCharIndex += col;
    }
    fullLineText += lText;
  }

  const { token, startCol } = extractTokenAtColumn(fullLineText, cursorCharIndex);
  if (!token) return null;

  // Check URL
  const url = detectUrl(token);
  if (url) {
    return {
      type: 'url',
      rawText: token,
      startCol,
      bufferY,
      length: token.length,
    };
  }

  // Check File Path
  const pathInfo = detectFilePath(token);
  if (pathInfo) {
    return {
      type: pathInfo.isDir ? 'directory' : 'file',
      rawText: token,
      cleanedPath: pathInfo.cleanPath,
      line: pathInfo.line,
      col: pathInfo.col,
      windowsPath: pathInfo.windowsPath,
      gitBashPath: pathInfo.gitBashPath,
      wslPath: pathInfo.wslPath,
      startCol,
      bufferY,
      length: token.length,
    };
  }

  return null;
}
