//! MD5, for `Row.TreeId`: existing `.tim` files key their row settings by it, so it has to match byte for byte.

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11,
    16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// The digest of `input`.
pub fn md5(input: &[u8]) -> [u8; 16] {
    let k: Vec<u32> = (0..64).map(|i| (((i as f64 + 1.0).sin().abs()) * 4294967296.0) as u32).collect();
    let (mut a0, mut b0, mut c0, mut d0) = (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);

    let mut message = input.to_vec();
    let bit_len = (input.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_le_bytes());

    for chunk in message.chunks_exact(64) {
        let m: Vec<u32> = chunk.chunks_exact(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let rotated = a.wrapping_add(f).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(rotated.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut out = [0u8; 16];
    out[..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

/// `BitConverter.ToString(hash).Replace("-", "")`: uppercase hex.
pub fn md5_hex_upper(input: &str) -> String {
    let mut s = String::with_capacity(32);
    for b in md5(input.as_bytes()) {
        s.push_str(&format!("{b:02X}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_digests() {
        assert_eq!(md5_hex_upper(""), "D41D8CD98F00B204E9800998ECF8427E");
        assert_eq!(md5_hex_upper("abc"), "900150983CD24FB0D6963F7D28E17F72");
        assert_eq!(md5_hex_upper("12345678901234567890123456789012345678901234567890123456789012345678901234567890"), "57EDF4A22BE3C955AC49DA2E2107B67A");
    }

    #[test]
    fn tree_ids_from_the_spec() {
        assert_eq!(md5_hex_upper("Arches"), "0A9E1587CEC47A218E3B7B1E84D5F22F");
        assert_eq!(md5_hex_upper("Arch 1Arches"), "D490FDFA296E6757D918AB22E8F6F39D");
    }
}
