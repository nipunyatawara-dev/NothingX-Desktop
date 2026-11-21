//! Nothing/CMF SPP framing. Protocol lineage: Radiance ear-web and Nothing X Mac.
//! Requests carry a whole-frame CRC-16/MODBUS. Device frames may carry a whole-frame
//! CRC, a payload-only CRC (e.g. CMF Buds Pro 2, CMF Buds 2a), or no CRC when control
//! bit 0x20 is clear (e.g. unsolicited CMF Buds Pro 2 events, Ear (1)). Never accept
//! a partial or corrupted frame as valid.

pub const MAX_PAYLOAD: usize = 255;
const HEADER: usize = 8;
const OVERHEAD: usize = 10;
const CTRL_UNUSED: u8 = 0x80;
const CTRL_CRC: u8 = 0x20;
const CTRL_STATUS_MASK: u8 = 0x1f;
const MAX_NO_CRC_STATUS: u8 = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub command: u16,
    pub operation: u8,
    pub status: u8,
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
                let control_lo = self.buffer[1];
                let control_hi = self.buffer[2];
                let cmd_hi = self.buffer[4];
                let has_crc = control_lo & CTRL_CRC != 0;
                let status = control_lo & CTRL_STATUS_MASK;
                if control_lo & CTRL_UNUSED != 0 || control_hi != 0x01 || self.buffer[6] != 0x00 {
                    self.buffer.remove(0);
                    continue;
                }
                if !has_crc
                    && (status > MAX_NO_CRC_STATUS || !matches!(cmd_hi, 0x40 | 0x60 | 0x70 | 0xe0))
                {
                    self.buffer.remove(0);
                    continue;
                }
                let payload_len = usize::from(self.buffer[5]);
                let body = HEADER + payload_len;
                let length = payload_len + if has_crc { OVERHEAD } else { HEADER };
                if self.buffer.len() < length {
                    break;
                }
                if has_crc {
                    let expected = u16::from_le_bytes([self.buffer[body], self.buffer[body + 1]]);
                    let valid = crc16(&self.buffer[..body]) == expected
                        || crc16(&self.buffer[HEADER..body]) == expected;
                    if !valid {
                        self.buffer.remove(0);
                        continue;
                    }
                }
                frames.push(Frame {
                    command: u16::from_le_bytes([self.buffer[3], cmd_hi]),
                    operation: self.buffer[7],
                    status,
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
                    status: 0,
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
                status: 0,
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
    #[test]
    fn payload_only_crc_frames_parse_and_reject_corruption() {
        // Real captures: CMF Buds Pro 2 / Buds 2a checksum only the payload.
        let battery = decode_hex("556001074005000202025f035f2ca2").unwrap();
        let firmware = decode_hex("5560014240080001312e302e312e3630d866").unwrap();
        let empty_ack = decode_hex("5560010f70000001ffff").unwrap();

        for split in 0..battery.len() {
            let mut parser = Parser::default();
            assert!(parser.push(&battery[..split]).is_empty());
            let frames = parser.push(&battery[split..]);
            assert_eq!(frames.len(), 1);
            assert_eq!(frames[0].command, 0x4007);
            assert_eq!(frames[0].operation, 2);
            assert_eq!(frames[0].payload, vec![2, 2, 0x5f, 3, 0x5f]);
        }

        let mut parser = Parser::default();
        let mut combined = firmware;
        combined.extend(&empty_ack);
        let frames = parser.push(&combined);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].command, 0x4042);
        assert_eq!(frames[0].payload, b"1.0.1.60");
        assert_eq!(frames[1].command, 0x700f);
        assert!(frames[1].payload.is_empty());

        let mut corrupt = battery;
        *corrupt.last_mut().unwrap() ^= 0xff;
        assert!(Parser::default().push(&corrupt).is_empty());
    }
    #[test]
    fn non_multiframe_and_no_crc_device_frames_parse() {
        // Reply with control 0x0120 (CRC set, multi-frame clear).
        let reply_0120 = decode_hex("5520011e4006000301070002040011ba").unwrap();
        let frames = Parser::default().push(&reply_0120);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].command, 0x401e);
        assert_eq!(frames[0].payload, vec![1, 7, 0, 2, 4, 0]);

        // CMF Buds Pro 2 unsolicited ANC events carry control 0x0100 (no CRC).
        let event_3b = decode_hex("55000103e0030000010500").unwrap();
        let event_2b = decode_hex("55000103e00200000107").unwrap();
        let mut stream = event_3b;
        stream.extend(event_2b);
        let frames = Parser::default().push(&stream);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].command, 0xe003);
        assert_eq!(frames[0].payload, vec![1, 5, 0]);
        assert_eq!(frames[1].command, 0xe003);
        assert_eq!(frames[1].payload, vec![1, 7]);

        // Ear (1) reply without CRC (control 0x0140).
        let ear1 = decode_hex("5540010740030000010255").unwrap();
        let frames = Parser::default().push(&ear1);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].command, 0x4007);
        assert_eq!(frames[0].payload, vec![1, 2, 0x55]);
    }
}
