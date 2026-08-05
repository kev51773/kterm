use regex::Regex;
use std::collections::VecDeque;
use std::sync::Mutex;

pub const DEFAULT_RING_BUFFER_CAPACITY: usize = 256 * 1024; // 256 KB

#[derive(Debug)]
pub struct RingBuffer {
    capacity: usize,
    buffer: Mutex<VecDeque<u8>>,
    total_bytes_written: Mutex<usize>,
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl RingBuffer {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_RING_BUFFER_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: Mutex::new(VecDeque::with_capacity(capacity)),
            total_bytes_written: Mutex::new(0),
        }
    }

    pub fn append(&self, data: &[u8]) {
        let mut buf = self.buffer.lock().unwrap();
        for &byte in data {
            if buf.len() >= self.capacity {
                buf.pop_front();
            }
            buf.push_back(byte);
        }
        let mut total = self.total_bytes_written.lock().unwrap();
        *total += data.len();
    }

    pub fn get_total_bytes_written(&self) -> usize {
        *self.total_bytes_written.lock().unwrap()
    }

    pub fn get_raw_bytes(&self) -> Vec<u8> {
        let buf = self.buffer.lock().unwrap();
        buf.iter().copied().collect()
    }

    pub fn get_text(&self, strip_ansi_codes: bool) -> String {
        let bytes = self.get_raw_bytes();
        let raw_str = String::from_utf8_lossy(&bytes);
        if strip_ansi_codes {
            strip_ansi(&raw_str)
        } else {
            raw_str.to_string()
        }
    }

    pub fn get_text_from_offset(&self, start_offset: usize, strip_ansi_codes: bool) -> String {
        let current_total = self.get_total_bytes_written();
        if current_total <= start_offset {
            return String::new();
        }
        let bytes_since = current_total - start_offset;
        let buf = self.buffer.lock().unwrap();
        let len = buf.len();
        let take_bytes = bytes_since.min(len);
        let start_idx = len - take_bytes;
        let slice: Vec<u8> = buf.iter().skip(start_idx).copied().collect();
        let raw_str = String::from_utf8_lossy(&slice);
        if strip_ansi_codes {
            strip_ansi(&raw_str)
        } else {
            raw_str.to_string()
        }
    }

    pub fn read_tail_lines(&self, n: usize, strip_ansi_codes: bool) -> Vec<String> {
        let text = self.get_text(strip_ansi_codes);
        let lines: Vec<&str> = text.lines().collect();

        // Skip leading blank lines from PTY screen buffer initialization
        let start_idx = lines.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
        let active_lines = &lines[start_idx..];

        let total = active_lines.len();
        if total == 0 {
            return Vec::new();
        }
        let start = if total > n { total - n } else { 0 };
        active_lines[start..].iter().map(|s| s.to_string()).collect()
    }

    #[allow(dead_code)]
    pub fn contains_pattern(&self, pattern: &str) -> bool {
        self.contains_pattern_from_offset(0, pattern)
    }

    pub fn contains_pattern_from_offset(&self, start_offset: usize, pattern: &str) -> bool {
        if pattern.is_empty() {
            return true;
        }
        let clean_text = if start_offset > 0 {
            self.get_text_from_offset(start_offset, true)
        } else {
            self.get_text(true)
        };
        if clean_text.is_empty() {
            return false;
        }
        if clean_text.contains(pattern) {
            return true;
        }
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&clean_text) {
                return true;
            }
        }
        false
    }

    pub fn matches_prompt(&self) -> bool {
        let clean_text = self.get_text(true);
        let trimmed = clean_text.trim_end();
        if trimmed.is_empty() {
            return false;
        }
        let last_line = trimmed.lines().last().unwrap_or("").trim();

        if last_line.ends_with('>')
            || last_line.ends_with('$')
            || last_line.ends_with('#')
            || last_line.contains("PS ")
        {
            return true;
        }

        false
    }
}

pub fn strip_ansi(input: &str) -> String {
    // Convert cursor positioning sequences (\x1b[H, \x1b[24;1H) to newlines so lines split cleanly
    let re_cursor = Regex::new(r"\x1B\[\d*;?\d*[Hf]").unwrap();
    let with_newlines = re_cursor.replace_all(input, "\n");

    let re_csi = Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").unwrap();
    let re_osc = Regex::new(r"\x1B\].*?(?:\x07|\x1B\\)").unwrap();
    let no_osc = re_osc.replace_all(&with_newlines, "");
    let no_csi = re_csi.replace_all(&no_osc, "");
    no_csi.replace('\r', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_capacity() {
        let rb = RingBuffer::with_capacity(10);
        rb.append(b"0123456789ABCDE");
        let bytes = rb.get_raw_bytes();
        assert_eq!(bytes.len(), 10);
        assert_eq!(String::from_utf8(bytes).unwrap(), "56789ABCDE");
    }

    #[test]
    fn test_strip_ansi() {
        let ansi_str = "\x1b[32mHello\x1b[0m World!";
        assert_eq!(strip_ansi(ansi_str), "Hello World!");
    }

    #[test]
    fn test_read_tail_lines() {
        let rb = RingBuffer::new();
        rb.append(b"line1\nline2\nline3\nline4\nline5");
        let tail = rb.read_tail_lines(3, true);
        assert_eq!(tail, vec!["line3", "line4", "line5"]);
    }

    #[test]
    fn test_contains_pattern() {
        let rb = RingBuffer::new();
        rb.append(b"Build Status: \x1b[32mSUCCESS\x1b[0m in 42s");
        assert!(rb.contains_pattern("SUCCESS"));
        assert!(rb.contains_pattern("Build Status"));
        assert!(rb.contains_pattern(r"SUCCESS in \d+s"));
    }

    #[test]
    fn test_contains_pattern_from_offset() {
        let rb = RingBuffer::new();
        rb.append(b"Old Output: SUCCESS\n");
        let offset = rb.get_total_bytes_written();

        assert!(!rb.contains_pattern_from_offset(offset, "SUCCESS"));

        rb.append(b"New Output: SUCCESS\n");
        assert!(rb.contains_pattern_from_offset(offset, "SUCCESS"));
    }

    #[test]
    fn test_matches_prompt() {
        let rb = RingBuffer::new();
        rb.append(b"PS C:\\Users\\Kev> ");
        assert!(rb.matches_prompt());

        let rb2 = RingBuffer::new();
        rb2.append(b"user@ubuntu:~$ ");
        assert!(rb2.matches_prompt());
    }
}
