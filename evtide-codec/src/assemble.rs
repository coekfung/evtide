/// Assembles fixed-width little-endian words from arbitrary byte chunks.
///
/// Buffers partial bytes in `pending` / `pending_len` across calls. Each
/// complete `N`-byte word is passed to `on_word`.
///
/// Returns the number of bytes consumed from `bytes`.
pub(crate) fn assemble_words<const N: usize>(
    pending: &mut [u8; N],
    pending_len: &mut u8,
    bytes: &[u8],
    mut on_word: impl FnMut([u8; N]),
) -> usize {
    let mut consumed = 0;
    let needed = N - *pending_len as usize;

    if bytes.len() >= needed {
        pending[*pending_len as usize..N].copy_from_slice(&bytes[..needed]);
        on_word(*pending);
        consumed += needed;
        *pending_len = 0;

        let remainder = &bytes[needed..];
        let mut chunks = remainder.chunks_exact(N);
        for chunk in chunks.by_ref() {
            on_word(chunk.try_into().unwrap());
        }
        consumed += remainder.len() - chunks.remainder().len();

        let leftover = chunks.remainder();
        pending[..leftover.len()].copy_from_slice(leftover);
        *pending_len = leftover.len() as u8;
    } else {
        pending[*pending_len as usize..*pending_len as usize + bytes.len()].copy_from_slice(bytes);
        *pending_len += bytes.len() as u8;
    }

    consumed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u32_even_boundary() {
        let mut pending = [0u8; 4];
        let mut plen = 0;
        let bytes = 0x01020304u32.to_le_bytes();
        let mut words = Vec::new();
        let c = assemble_words::<4>(&mut pending, &mut plen, &bytes, |w| words.push(w));
        assert_eq!(c, 4);
        assert_eq!(plen, 0);
        assert_eq!(words.len(), 1);
    }

    #[test]
    fn u32_split_across_calls() {
        let mut pending = [0u8; 4];
        let mut plen = 0;
        let bytes = 0xAABBCCDDu32.to_le_bytes();
        let mut words = Vec::new();

        let c1 = assemble_words::<4>(&mut pending, &mut plen, &bytes[..3], |w| words.push(w));
        assert_eq!(c1, 0);
        assert_eq!(plen, 3);

        let c2 = assemble_words::<4>(&mut pending, &mut plen, &bytes[3..], |w| words.push(w));
        assert_eq!(c2, 1);
        assert_eq!(plen, 0);
        assert_eq!(words.len(), 1);
        assert_eq!(u32::from_le_bytes(words[0]), 0xAABBCCDD);
    }

    #[test]
    fn u64_multiple_words() {
        let mut pending = [0u8; 8];
        let mut plen = 0;
        let a = 0x0102030405060708u64.to_le_bytes();
        let b = 0x1112131415161718u64.to_le_bytes();
        let mut bytes = Vec::from(a);
        bytes.extend_from_slice(&b);

        let mut words = Vec::new();
        let c = assemble_words::<8>(&mut pending, &mut plen, &bytes, |w| words.push(w));
        assert_eq!(c, 16);
        assert_eq!(words.len(), 2);
    }
}
