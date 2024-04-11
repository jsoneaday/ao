use std::io::Write;
use crate::{data_item::MAX_TAG_BYTES, errors::ArBundleErrors};

pub struct Tag {
    pub name: String,
    pub value: String
}

pub struct AVSCTap {
    buf: Vec<u8>,
    pos: i64
}

impl AVSCTap {
    pub fn new(buf: Option<Vec<u8>>, pos: Option<i64>) -> Self {
        Self {
            buf: if buf.is_none() { Vec::with_capacity(MAX_TAG_BYTES) } else { buf.unwrap() },
            pos: if pos.is_none() { 0 } else { pos.unwrap() }
        }
    }

    pub fn write_tags(&mut self, tags: &Vec<Tag>) -> Result<(), ArBundleErrors> {
        let n: i64 = tags.len() as i64;

        if n > 0 {
            self.write_long(n);
            for i in 0..n {
                let tag = tags.get(i as usize);
                if tag.is_none() || tag.unwrap().name.is_empty() || tag.unwrap().value.is_empty() {
                    return Err(ArBundleErrors::TagIsUndefinedOrEmpty)
                }
                self.write_string(&tag.unwrap().name);
                self.write_string(&tag.unwrap().value);
            }
        }
        self.write_long(0);

        Ok(())
    }

    pub fn write_long(&mut self, n: i64) -> Result<(), ArBundleErrors> {
        let mut f: f64;
        let mut m: i64;
    
        if n >= -1073741824 && n < 1073741824 {
            // Won't overflow, use integer arithmetic
            m = if n >= 0 { n << 1 } else { (!n << 1) | 1 };
            loop {
                match self.buf.write_all(&[m as u8 & 0x7f]) {
                    Ok(_) => {
                        m >>= 7;
                        if m == 0 {
                            break;
                        }
                        match self.buf.write_all(&[0x80 | (m as u8 & 0x7f)]) {
                            Ok(_) => {},
                            Err(e) => return Err(ArBundleErrors::IoFailure(e))
                        }
                    },
                    Err(e) => return Err(ArBundleErrors::IoFailure(e))
                }
            }
        } else {
            // Use slower floating-point arithmetic
            f = if n >= 0 { n as f64 * 2.0 } else { -(n as f64 * 2.0) - 1.0 };
            loop {
                match self.buf.write_all(&[f as u8 & 0x7f]) {
                    Ok(_) => {
                        f /= 128.0;
                        if f < 1.0 {
                            break;
                        }
                        match self.buf.write_all(&[0x80 | (f as u8 & 0x7f)]) {
                            Ok(_) => {},
                            Err(e) => return Err(ArBundleErrors::IoFailure(e))
                        }
                    },
                    Err(e) => return Err(ArBundleErrors::IoFailure(e))
                }
            }
        }
    
        self.pos += 1; // Update position (assuming it's a u8)
        Ok(())
    }

    pub fn write_string(&mut self, s: &str) -> Result<(), ArBundleErrors> {
        let len = s.len() as i64; // Length as i64 for write_long
        self.write_long(len)?; // Write string length

        let mut pos = self.pos;
        if pos + len > self.buf.len() as i64 {
            return Err(ArBundleErrors::IoFailure(std::io::Error::new(std::io::ErrorKind::Other, "Buffer overflow")));
        }

        if len > 64 {
            match self.buf.write_all(s.as_bytes()) {
                Ok(_) => {},
                Err(e) => return Err(ArBundleErrors::IoFailure(e))
            }
        } else {
            for c in s.chars() {
                let code = c as u32;
                if code < 0x80 {
                    self.buf[pos as usize] = code as u8;
                    pos += 1;
                } else if code < 0x800 {
                    self.buf[pos as usize] = (code >> 6) as u8 | 0xc0;
                    pos += 1;
                    self.buf[pos as usize] = (code & 0x3f) as u8 | 0x80;
                    pos += 1;
                } else if code >= 0xd800 && code < 0xe000 && (s.chars().nth(s.char_indices().next().unwrap().0 + 1).unwrap() as u32) >= 0xdc00 && (s.chars().nth(s.char_indices().next().unwrap().0 + 1).unwrap() as u32) < 0xe000 {
                    // Handle surrogate pairs (UTF-16)
                    let c2 = s.chars().nth(s.char_indices().next().unwrap().0 + 1).unwrap() as u32;
                    let codepoint = 0x10000 + ((code & 0x03ff) << 10) + (c2 & 0x03ff);
                    self.buf[pos as usize] = (codepoint >> 18) as u8 | 0xf0;
                    pos += 1;
                    self.buf[pos as usize] = ((codepoint >> 12) & 0x3f) as u8 | 0x80;
                    pos += 1;
                    self.buf[pos as usize] = ((codepoint >> 6) & 0x3f) as u8 | 0x80;
                    pos += 1;
                    self.buf[pos as usize] = (codepoint & 0x3f) as u8 | 0x80;
                    pos += 1;
                    // Skip the second character (already processed)
                } else {
                    self.buf[pos as usize] = (code >> 12) as u8 | 0xe0;
                    pos += 1;
                    self.buf[pos as usize] = ((code >> 6) & 0x3f) as u8 | 0x80;
                    pos += 1;
                    self.buf[pos as usize] = (code & 0x3f) as u8 | 0x80;
                    pos += 1;
                }
            }
        }

        self.pos = pos;
        Ok(())
    }

    pub fn to_buffer(&mut self) -> Result<Vec<u8>, ArBundleErrors> {        
        if self.pos > self.buf.len() as i64 { 
            return Err(ArBundleErrors::IoFailure(
                std::io::Error::new(std::io::ErrorKind::Other, format!("Too many tag bytes ({:?} > {})", self.pos, self.buf.len()))
            ));
        }
        
        let mut buffer: Vec<u8> = Vec::with_capacity(self.pos as usize);
        match buffer.write_all(&self.buf[..self.pos as usize]) {
            Ok(_) => {},
            Err(e) => return Err(ArBundleErrors::IoFailure(e))
        }
        return Ok(buffer);
    }

    pub fn serialize_tags(tags: &Vec<Tag>) -> Result<Vec<u8>, ArBundleErrors> {
        let mut tap = AVSCTap {
            buf: vec![],
            pos: 0
        };
        tap.write_tags(tags);
        return tap.to_buffer();
    }
}