import {
  detectUrl,
  detectFilePath,
  parsePathSuffix,
  toWindowsPath,
  toGitBashPath,
  toWslPath,
  formatCwdForProfile,
  extractTokenAtColumn,
} from './detector';

function assertEqual(actual: any, expected: any, message?: string) {
  if (actual !== expected) {
    throw new Error(`Assertion failed: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}. ${message || ''}`);
  }
}

console.log('[Test] Running detector unit tests...');

// 1. Test URL detection
assertEqual(detectUrl('https://github.com/foo/bar'), 'https://github.com/foo/bar');
assertEqual(detectUrl('http://localhost:3000/api/health'), 'http://localhost:3000/api/health');
assertEqual(detectUrl('www.google.com'), 'http://www.google.com');
assertEqual(detectUrl('"https://example.com"'), 'https://example.com');
assertEqual(detectUrl('not-a-url'), null);

// 2. Test Suffix & Prompt Parsing
const s1 = parsePathSuffix('src/main.ts:123:45');
assertEqual(s1.cleanPath, 'src/main.ts');
assertEqual(s1.line, 123);
assertEqual(s1.col, 45);

const s2 = parsePathSuffix('C:\\Users\\Kev\\file.py(42)');
assertEqual(s2.cleanPath, 'C:\\Users\\Kev\\file.py');
assertEqual(s2.line, 42);

const s3 = parsePathSuffix('user@hostname:~/project/file.ts$');
assertEqual(s3.cleanPath, '~/project/file.ts');

const s4 = parsePathSuffix('user@laptop:/var/log/syslog#');
assertEqual(s4.cleanPath, '/var/log/syslog');

// 3. Test Path Conversions
assertEqual(toWindowsPath('/c/Users/Kev/Desktop/Terminal'), 'C:\\Users\\Kev\\Desktop\\Terminal');
assertEqual(toWindowsPath('/mnt/c/Users/Kev/Desktop/Terminal'), 'C:\\Users\\Kev\\Desktop\\Terminal');

assertEqual(toGitBashPath('C:\\Users\\Kev\\Desktop\\Terminal'), '/c/Users/Kev/Desktop/Terminal');
assertEqual(toWslPath('C:\\Users\\Kev\\Desktop\\Terminal'), '/mnt/c/Users/Kev/Desktop/Terminal');

// 4. Test Profile-specific CWD formatting
assertEqual(formatCwdForProfile('C:\\Users\\Kev', 'powershell'), 'C:\\Users\\Kev');
assertEqual(formatCwdForProfile('C:\\Users\\Kev', 'cmd'), 'C:\\Users\\Kev');
assertEqual(formatCwdForProfile('C:\\Users\\Kev', 'wsl'), '/mnt/c/Users/Kev');
assertEqual(formatCwdForProfile('C:\\Users\\Kev', 'git-bash'), '/c/Users/Kev');
assertEqual(formatCwdForProfile('/c/Users/Kev', 'powershell'), 'C:\\Users\\Kev');

// 5. Test File Path Detection (Windows, WSL, POSIX, Tilde)
const p1 = detectFilePath('C:\\Users\\Kev\\Desktop\\file.txt');
if (!p1) throw new Error('Expected p1 to be non-null');
assertEqual(p1.windowsPath, 'C:\\Users\\Kev\\Desktop\\file.txt');
assertEqual(p1.gitBashPath, '/c/Users/Kev/Desktop/file.txt');

const p2 = detectFilePath('/mnt/c/Projects/app/src/index.ts:15');
if (!p2) throw new Error('Expected p2 to be non-null');
assertEqual(p2.cleanPath, '/mnt/c/Projects/app/src/index.ts');
assertEqual(p2.line, 15);
assertEqual(p2.windowsPath, 'C:\\Projects\\app\\src\\index.ts');

const p3 = detectFilePath('~/code/project/readme.md');
if (!p3) throw new Error('Expected p3 to be non-null');
assertEqual(p3.cleanPath, '~/code/project/readme.md');

const p4 = detectFilePath('/var/log/nginx/access.log');
if (!p4) throw new Error('Expected p4 to be non-null');
assertEqual(p4.cleanPath, '/var/log/nginx/access.log');

// 6. Test Token Extraction
const text = 'Error at src/components/ContextMenu.ts:42 in main';
assertEqual(extractTokenAtColumn(text, 10).token, 'src/components/ContextMenu.ts:42');

console.log('[Test] All detector unit tests passed successfully!');
