use crate::errors::ArBundleErrors;

pub fn byte_array_to_long(byte_array: &[u8]) -> i64 {
    let mut value: i64 = 0;
    for i in (0..byte_array.len() - 1).rev() {
      value = value * 256 + byte_array[i] as i64;
    }
    return value;
}

pub fn short_to_2_byte_array(mut long: i64) -> Result<[u8; 2], ArBundleErrors> {
    if long > (2 ^ (32 - 1)) { 
        return Err(ArBundleErrors::IoFailure(std::io::Error::new(std::io::ErrorKind::Other, "Short too long")));
    }
    // we want to represent the input as a 8-bytes array
    let mut byte_array = [0, 0];
  
    for index in 0..byte_array.len() {
      let byte = long & 0xff;
      byte_array[index] = byte as u8;
      long >>= 8;
    }
  
    Ok(byte_array)
}

pub fn long_to_8_byte_array(long: i64) -> [u8; 8] {
    // we want to represent the input as a 8-bytes array
    let mut byte_array = [0u8; 8];
    let mut long = long;

    for i in 0..8 {
        let byte = long & 0xff;
        byte_array[i] = byte as u8;
        long = (long - byte) / 256;
    }

    byte_array
}