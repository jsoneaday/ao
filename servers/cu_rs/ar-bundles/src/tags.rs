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
    pub fn new(buf: Option<Vec<u8>>, pos: i64) -> Self {
        Self {
            buf: if buf.is_none() { Vec::with_capacity(MAX_TAG_BYTES) } else { buf.unwrap() },
            pos
        }
    }

    pub fn write_tags(&self, tags: Vec<Tag>) -> Result<(), ArBundleErrors> {
        let n: i64 = tags.len() as i64;

        if n > 0 {
            self.write_long(n);
            for i in 0..n {
                let tag = tags.get(i as usize);
                if tag.is_none() || tag.unwrap().name.is_empty() || tag.unwrap().value.is_empty() {
                    return Err(ArBundleErrors::TagIsUndefinedOrEmpty)
                }
                self.write_string(tag.unwrap().name);
                self.write_string(tag.unwrap().value);
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

}