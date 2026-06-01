use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

pub struct MapleCodec;

impl MapleCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MapleCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for MapleCodec {
    type Item = (u16, Bytes);
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }
        let packet_len = u16::from_le_bytes([src[0], src[1]]) as usize;
        if src.len() < 2 + packet_len {
            return Ok(None);
        }
        src.advance(2);
        let opcode = u16::from_le_bytes([src[0], src[1]]);
        src.advance(2);
        let payload_len = packet_len - 2;
        let payload = src.split_to(payload_len).freeze();
        Ok(Some((opcode, payload)))
    }
}

impl Encoder<(u16, &[u8])> for MapleCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: (u16, &[u8]), dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (opcode, payload) = item;
        let total = 2 + payload.len();
        dst.put_u16_le(total as u16);
        dst.put_u16_le(opcode);
        dst.put_slice(payload);
        Ok(())
    }
}
