//! Nothing/CMF SPP framing. Protocol lineage: Radiance ear-web and Nothing X Mac.
//! All production frames require a CRC. Never accept a partial frame as valid.

pub const MAX_PAYLOAD: usize = 255;
const HEADER: usize = 8;
const OVERHEAD: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub command: u16,
    pub operation: u8,
    pub payload: Vec<u8>,
}

pub fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xffff;
    for &byte in bytes {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xa001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

pub fn encode(command: u16, payload: &[u8], operation: u8) -> Result<Vec<u8>, &'static str> {
    if payload.len() > MAX_PAYLOAD {
        return Err("Bluetooth payload exceeds 255 bytes");
    }
    let [low, high] = command.to_le_bytes();
    let mut bytes = vec![
        0x55,
        0x60,
        0x01,
        low,
        high,
        payload.len() as u8,
        0,
        operation,
    ];
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(&crc16(&bytes).to_le_bytes());
    Ok(bytes)
}

#[derive(Default)]
pub struct Parser {
    buffer: Vec<u8>,
}

impl Parser {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Frame> {
        let mut frames = Vec::new();
        // Process incrementally: a hostile input chunk cannot grow retained memory.
        for &byte in chunk {
            if self.buffer.is_empty() && byte != 0x55 {
                continue;
            }
            self.buffer.push(byte);
            loop {
                if self.buffer.first() != Some(&0x55) {
                    let Some(start) = self.buffer.iter().position(|b| *b == 0x55) else {
                        self.buffer.clear();
                        break;
                    };
                    self.buffer.drain(..start);
                }
                if self.buffer.len() < HEADER {
                    break;
                }
                if self.buffer[1] != 0x60 || self.buffer[2] != 0x01 || self.buffer[6] != 0x00 {
                    self.buffer.remove(0);
                    continue;
                }
                let length = OVERHEAD + usize::from(self.buffer[5]);
                if self.buffer.len() < length {
                    break;
                }
                let body = length - 2;
                let expected = u16::from_le_bytes([self.buffer[body], self.buffer[body + 1]]);
                if crc16(&self.buffer[..body]) != expected {
                    self.buffer.remove(0);
                    continue;
                }
                frames.push(Frame {
                    command: u16::from_le_bytes([self.buffer[3], self.buffer[4]]),
                    operation: self.buffer[7],
                    payload: self.buffer[HEADER..body].to_vec(),
                });
                self.buffer.drain(..length);
            }
        }
        frames
    }
}

pub fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) || hex.len() > 8192 {
        return None;
    }
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let high = char::from(pair[0]).to_digit(16)?;
            let low = char::from(pair[1]).to_digit(16)?;
            Some(((high << 4) | low) as u8)
        })
        .collect()
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(result, "{b:02x}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_crc_vector() {
        assert_eq!(crc16(b"123456789"), 0x4b37);
    }
    #[test]
    fn transmitted_packets_match_upstream_reference_vectors() {
        assert_eq!(
            hex(&encode(0xc007, &[], 1).unwrap()),
            "55600107c0000001acdf"
        );
        assert_eq!(
            hex(&encode(0xf00f, &[1, 4, 0], 2).unwrap()),
            "5560010ff0030002010400fac3"
        );
    }
    #[test]
    fn every_fragment_boundary_waits_for_crc() {
        let bytes = encode(0x4007, &[3, 2, 87, 3, 0xd0, 4, 42], 7).unwrap();
        for split in 0..bytes.len() {
            let mut parser = Parser::default();
            assert!(parser.push(&bytes[..split]).is_empty());
            assert_eq!(
                parser.push(&bytes[split..]),
                vec![Frame {
                    command: 0x4007,
                    operation: 7,
                    payload: vec![3, 2, 87, 3, 0xd0, 4, 42]
                }]
            );
        }
    }
    #[test]
    fn corruption_and_noise_do_not_become_device_updates() {
        let valid = encode(0x401e, &[1, 7, 0], 1).unwrap();
        let mut broken = valid.clone();
        broken[9] ^= 1;
        let mut input = vec![0, 1, 2];
        input.extend(broken);
        input.extend(&valid);
        input.extend(&valid);
        let mut parser = Parser::default();
        assert_eq!(parser.push(&input).len(), 2);
        assert!(parser.buffer.len() <= MAX_PAYLOAD + OVERHEAD);
    }
    #[test]
    fn stray_start_byte_does_not_stall_following_valid_frame() {
        let valid = encode(0x4007, &[1, 2, 80], 1).unwrap();
        let mut input = vec![0x55, 0x00, 0x00, 0x00, 0x00, 255, 0x00, 0x00];
        input.extend(&valid);
        let mut parser = Parser::default();
        assert_eq!(
            parser.push(&input),
            vec![Frame {
                command: 0x4007,
                operation: 1,
                payload: vec![1, 2, 80],
            }]
        );
    }
    #[test]
    fn oversized_payload_and_bad_hex_are_rejected() {
        assert!(encode(1, &[0; 256], 0).is_err());
        assert!(decode_hex("xx").is_none());
        assert!(decode_hex("0").is_none());
        assert!(decode_hex("é").is_none());
    }
}
