use std::io::Write;
use crate::{data_item::MAX_TAG_BYTES, errors::ArBundleErrors};

pub struct Tag {
    pub name: String,
    pub value: String
}

pub struct AVSCTap {
    buf: Vec<u8>,
    pos: usize
}

impl AVSCTap {
    pub fn new(buf: Option<Vec<u8>>, pos: Option<usize>) -> Self {
        Self {
            buf: if buf.is_none() { Vec::with_capacity(MAX_TAG_BYTES) } else { buf.unwrap() },
            pos: if pos.is_none() { 0 } else { pos.unwrap() }
        }
    }

    pub fn write_tags(&mut self, tags: &Vec<Tag>) -> Result<(), ArBundleErrors> {
        let n = tags.len();

        if n > 0 {
            self.write_long(n as i64);
            for i in 0..n {
                let tag = tags.get(i);
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
                self.buf[self.pos as usize] = (m & 0x7f) as u8;
                m >>= 7;
                if m == 0 && (self.buf[self.pos as usize] & 0x80 == 0) {
                    break;
                }
                self.pos += 1;
            }
        } else {
            // Use slower floating-point arithmetic
            f = if n >= 0 { n as f64 * 2.0 } else { -n as f64 * 2.0 - 1.0 };
            loop {
                self.buf[self.pos as usize] = (f as i32 & 0x7f) as u8;
                f /= 128.0;
                if f < 1.0 && (self.buf[self.pos as usize] & 0x80 == 0) {
                    break;
                }
                self.pos += 1;
            }
        }
    
        self.pos += 1; // Update position (assuming it's a u8)
        Ok(())
    }

    pub fn write_string(&mut self, s: &str) -> Result<(), ArBundleErrors> {
        let len = s.len();
        let buf = &mut self.buf;
        self.write_long(len as i64)?; 

        let mut pos = self.pos;
        self.pos += len;
        if self.pos > buf.len() {
            return Err(ArBundleErrors::IoFailure(std::io::Error::new(std::io::ErrorKind::Other, "Buffer overflow")));
        }

        if len > 64 {
            buf[pos..pos + len].copy_from_slice(s.as_bytes());
        } else {
            let mut c2: u32 = 0;
            let mut i = 0;
            for c in s.chars() {                
                let mut c1 = c as u32;                

                if c1 < 0x80 {
                    pos += 1;
                    self.buf[pos] = c1 as u8;                    
                } else if c1 < 0x800 {
                    pos += 1;
                    self.buf[pos] = (c1 >> 6) as u8 | 0xc0;
                    pos += 1;
                    self.buf[pos] = (c1 & 0x3f) as u8 | 0x80;
                } else if c1 & 0xfc00 == 0xd800 {
                    c2 = if s.chars().nth(i + 1).is_some() { s.chars().nth(i + 1).unwrap() as u32 } else { 0 };
                    c2 = c2 & 0xfc00;
                    if c2 == 0xdc00 {
                        c1 = 0x10000 + ((c1 & 0x03ff) << 10) + (c2 & 0x03ff);
                        i += 1;
                        pos += 1;
                        self.buf[pos] = (c1 >> 18) as u8 | 0xf0;
                        pos += 1;
                        self.buf[pos] = ((c1 >> 12) & 0x3f) as u8 | 0x80;
                        pos += 1;
                        self.buf[pos] = ((c1 >> 6) & 0x3f) as u8 | 0x80;
                        pos += 1;
                        self.buf[pos] = (c1 & 0x3f) as u8 | 0x80;
                    }                    
                } else {
                    pos += 1;
                    self.buf[pos] = (c1 >> 12) as u8 | 0xe0;
                    pos += 1;
                    self.buf[pos] = ((c1 >> 6) & 0x3f) as u8 | 0x80;
                    pos += 1;
                    self.buf[pos] = (c1 & 0x3f) as u8 | 0x80;                                        
                }   
                i += 1;             
            }
        }

        self.buf = *buf;
        Ok(())
    }

    pub fn to_buffer(&mut self) -> Result<Vec<u8>, ArBundleErrors> {        
        if self.pos > self.buf.len() { 
            return Err(ArBundleErrors::IoFailure(
                std::io::Error::new(std::io::ErrorKind::Other, format!("Too many tag bytes ({:?} > {})", self.pos, self.buf.len()))
            ));
        }
        
        let mut buffer: Vec<u8> = Vec::with_capacity(self.pos);
        match buffer.write_all(&self.buf[..self.pos]) {
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