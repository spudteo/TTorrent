#[derive(Debug, Clone, Copy)]
pub struct Handshake {
    pstrlen: u8,
    pstr: [u8; 19],
    reserved: [u8; 8],
    pub info_hash: [u8; 20],
    peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(info_hash: [u8; 20], peer_id: &[u8; 20]) -> Self {
        let p = *b"BitTorrent protocol";
        let plen = p.len() as u8;
        Self {
            pstrlen: plen,
            pstr: p,
            reserved: [0; 8],
            info_hash,
            peer_id: peer_id.clone(),
        }
    }

    pub fn parse(input_bytes: [u8; 68]) -> Self {
        let pstrlen = input_bytes[0];
        let mut pstr = [0u8; 19];
        pstr.copy_from_slice(&input_bytes[1..20]);
        let mut reserved = [0u8; 8];
        reserved.copy_from_slice(&input_bytes[20..28]);
        let mut info_hash = [0u8; 20];
        info_hash.copy_from_slice(&input_bytes[28..48]);
        let mut peer_id = [0u8; 20];
        peer_id.copy_from_slice(&input_bytes[48..68]);
        Self {
            pstrlen,
            pstr,
            reserved,
            info_hash,
            peer_id,
        }
    }

    pub fn to_bytes(self: Self) -> [u8; 68] {
        let mut out = [0u8; 68];
        let mut pos = 1;
        out[0] = self.pstrlen;
        out[pos..pos + self.pstr.len()].copy_from_slice(&self.pstr);
        pos += self.pstr.len();
        out[pos..pos + self.reserved.len()].copy_from_slice(&self.reserved);
        pos += self.reserved.len();
        out[pos..pos + self.info_hash.len()].copy_from_slice(&self.info_hash);
        pos += self.info_hash.len();
        out[pos..pos + self.peer_id.len()].copy_from_slice(&self.peer_id);
        pos += self.peer_id.len();
        out
    }
}
