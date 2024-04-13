use std::io::Write;
use crate::{data_item::MAX_TAG_BYTES, errors::ArBundleErrors};

pub struct Tag {
    pub name: Option<String>,
    pub value: Option<String>
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
            _ = self.write_long(n as i64);
            for i in 0..n {
                let tag = tags.get(i);
                if tag.is_none() || tag.unwrap().name.is_none() || tag.unwrap().value.is_none() {
                    return Err(ArBundleErrors::TagIsUndefinedOrEmpty)
                }
                _ = self.write_string(if tag.unwrap().name.is_none() { "" } else { &tag.unwrap().name.as_ref().unwrap() });
                _ = self.write_string(if tag.unwrap().value.is_none() { "" } else { &tag.unwrap().value.as_ref().unwrap() });
            }
        }
        _ = self.write_long(0);

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
        self.write_long(len as i64)?; 
        let buf = &mut self.buf;

        let mut pos = self.pos;
        self.pos += len;
        if self.pos > buf.len() {
            return Err(ArBundleErrors::IoFailure(std::io::Error::new(std::io::ErrorKind::Other, "Buffer overflow")));
        }

        if len > 64 {
            buf[pos - len..pos].copy_from_slice(s.as_bytes());
        } else {
            #[allow(unused)]
            let mut c2: u32 = 0;
            let mut i = 0;
            for c in s.chars() {                
                let mut c1 = c as u32;                

                if c1 < 0x80 {
                    pos += 1;
                    buf[pos] = c1 as u8;                    
                } else if c1 < 0x800 {
                    pos += 1;
                    buf[pos] = (c1 >> 6) as u8 | 0xc0;
                    pos += 1;
                    buf[pos] = (c1 & 0x3f) as u8 | 0x80;
                } else if c1 & 0xfc00 == 0xd800 {
                    c2 = if s.chars().nth(i + 1).is_some() { 
                        if s.chars().nth(i + 1).is_some() {
                            s.chars().nth(i + 1).unwrap() as u32 
                        } else {
                            0
                        }
                    } else { 0 };
                    c2 = c2 & 0xfc00;
                    if c2 == 0xdc00 {
                        c1 = 0x10000 + ((c1 & 0x03ff) << 10) + (c2 & 0x03ff);
                        i += 1;
                        pos += 1;
                        buf[pos] = (c1 >> 18) as u8 | 0xf0;
                        pos += 1;
                        buf[pos] = ((c1 >> 12) & 0x3f) as u8 | 0x80;
                        pos += 1;
                        buf[pos] = ((c1 >> 6) & 0x3f) as u8 | 0x80;
                        pos += 1;
                        buf[pos] = (c1 & 0x3f) as u8 | 0x80;
                    }                    
                } else {
                    pos += 1;
                    buf[pos] = (c1 >> 12) as u8 | 0xe0;
                    pos += 1;
                    buf[pos] = ((c1 >> 6) & 0x3f) as u8 | 0x80;
                    pos += 1;
                    buf[pos] = (c1 & 0x3f) as u8 | 0x80;                                        
                }   
                i += 1;             
            }
        }

        self.buf = buf.clone();
        Ok(())
    }

    fn read_long(&mut self) -> Option<i64> {
        let mut n = 0;
        let mut k = 0;
        let buf = &self.buf;
        let mut b: u8;
        let mut h: u8;
        let mut f: i32;
        let mut fk: i32;
    
        loop {
            if self.pos >= buf.len() {
                return None;
            }

            self.pos += 1;
            b = buf[self.pos];
            h = b & 0x80;
            n |= (b as i32 & 0x7f) << k;
            k += 7;
            if h == 0 || k >= 28 {
                break;
            }
        }
    
        if h != 0 {
          // Switch to float arithmetic, otherwise we might overflow.
          f = n;
          fk = 268435456; // 2 ** 28.
          loop {
            if self.pos >= buf.len() {
                return None;
            }
            self.pos += 1;
            b = buf[self.pos];
            f += (b as i32 & 0x7f) * fk;
            fk *= 128;

            if b & 0x80 == 0 {
                break;
            }
          }
          return Some(((if f % 2 != 0 { -(f + 1) } else { f }) / 2) as i64);
        }
    
        Some(((n >> 1) ^ -(n & 1)) as i64)
    }

    pub fn skip_long(&mut self) {
        let buf = &self.buf;

        loop {
            self.pos += 1;
            
            if buf[self.pos] & 0x80 == 0 {
                break;
            }
        }
    }
  
    pub fn read_tags(&mut self) -> Vec<Tag> {
        // var items = this.itemsType;
        let mut val: Vec<Tag> = vec![];
        #[allow(unused)]
        let mut n = 0;
        loop {
            n = if self.read_long().is_none() { 0 } else {self.read_long().unwrap() };
            if n < 0 {
                n = -n;
                self.skip_long(); // Skip size.
            }
            for _ in (0..n).rev() {
                let name = self.read_string();
                let value = self.read_string();
                val.push(Tag { 
                    name: if name.is_ok() { name.unwrap() } else { Some("".to_string()) }, 
                    value: if value.is_ok() { value.unwrap() } else { Some("".to_string()) }
                });
            }
            if n == 0 {
                break;
            }
        }
        return val;
    }

    pub fn read_string(&mut self) -> Result<Option<String>, ArBundleErrors> {
        let len = if self.read_long().is_none() { 0 } else { self.read_long().unwrap() };
        let pos = self.pos;
        self.pos += len as usize;
        if self.pos > self.buf.len() {
          return Ok(None);
        }
        match String::from_utf8(self.buf[pos..pos + len as usize].to_vec()) {
            Ok(str) => Ok(Some(str)),
            Err(_) => Err(ArBundleErrors::IoFailure(
                std::io::Error::new(std::io::ErrorKind::Other,
                "Vec<u8> to string error".to_string())
            ))
        }
    }
}

pub fn deserialize_tags(tags_buffer: Vec<u8>) -> Vec<Tag> {
    let mut tap = AVSCTap {
        buf: tags_buffer,
        pos: 0
    };
    return tap.read_tags();
}

pub fn serialize_tags(tags: &Vec<Tag>) -> Result<Vec<u8>, ArBundleErrors> {
    let mut tap = AVSCTap {
        buf: vec![],
        pos: 0
    };
    _ = tap.write_tags(tags);
    return tap.to_buffer();
}