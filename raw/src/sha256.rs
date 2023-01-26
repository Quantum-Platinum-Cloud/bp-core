// BP Core Library implementing LNP/BP specifications & standards related to
// bitcoin protocol
//
// Written in 2018 by
//     Andrew Poelstra <apoelstra@wpsoftware.net>
// Written in 2023 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the Apache 2.0 License
// along with this software.
// If not, see <https://opensource.org/licenses/Apache-2.0>.

use std::cmp;

const BLOCK_SIZE: usize = 64;

macro_rules! ch ( ($x:expr, $y:expr, $z:expr) => ($z ^ ($x & ($y ^ $z))) );
macro_rules! maj ( ($x:expr, $y:expr, $z:expr) => (($x & $y) | ($z & ($x | $y))) );
macro_rules! _sigma0 ( ($x:expr) => ($x.rotate_left(30) ^ $x.rotate_left(19) ^ $x.rotate_left(10)) );
macro_rules! _sigma1 ( ($x:expr) => ($x.rotate_left(26) ^ $x.rotate_left(21) ^ $x.rotate_left(7)) );
macro_rules! sigma0  ( ($x:expr) => ($x.rotate_left(25) ^ $x.rotate_left(14) ^ ($x >> 3)) );
macro_rules! sigma1  ( ($x:expr) => ($x.rotate_left(15) ^ $x.rotate_left(13) ^ ($x >> 10)) );

macro_rules! round(
    // first round
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr, $k:expr, $w:expr) => (
        let t1 = $h.wrapping_add(_sigma1!($e)).wrapping_add(ch!($e, $f, $g)).wrapping_add($k).wrapping_add($w);
        let t2 = _sigma0!($a).wrapping_add(maj!($a, $b, $c));
        $d = $d.wrapping_add(t1);
        $h = t1.wrapping_add(t2);
    );
    // later rounds we reassign $w before doing the first-round computation
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr, $k:expr, $w:expr, $w1:expr, $w2:expr, $w3:expr) => (
        $w = $w.wrapping_add(sigma1!($w1)).wrapping_add($w2).wrapping_add(sigma0!($w3));
        round!($a, $b, $c, $d, $e, $f, $g, $h, $k, $w);
    )
);

/// Engine to compute SHA256 hash function.
#[derive(Clone)]
pub struct Sha256 {
    buffer: [u8; BLOCK_SIZE],
    h: [u32; 8],
    length: usize,
}

impl Default for Sha256 {
    fn default() -> Self {
        Sha256 {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f,
                0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
            length: 0,
            buffer: [0; BLOCK_SIZE],
        }
    }
}

impl Sha256 {
    /// Create a new [`Sha256`] from a midstate
    ///
    /// # Panics
    ///
    /// If `length` is not a multiple of the block size.
    pub fn from_midstate(midstate: [u8; 32], length: usize) -> Self {
        assert_eq!(
            length % BLOCK_SIZE,
            0,
            "length is no multiple of the block size"
        );

        let mut ret = [0; 8];
        for (ret_val, midstate_bytes) in
            ret.iter_mut().zip(midstate[..].chunks_exact(4))
        {
            *ret_val = u32::from_be_bytes(
                midstate_bytes.try_into().expect("4 byte slice"),
            );
        }

        Self {
            buffer: [0; BLOCK_SIZE],
            h: ret,
            length,
        }
    }

    pub fn midstate(&self) -> [u8; 32] {
        let mut ret = [0; 32];
        for (val, ret_bytes) in self.h.iter().zip(ret.chunks_exact_mut(4)) {
            ret_bytes.copy_from_slice(&val.to_be_bytes());
        }
        ret
    }

    pub fn input(&mut self, mut inp: &[u8]) {
        while !inp.is_empty() {
            let buf_idx = self.length % BLOCK_SIZE;
            let rem_len = BLOCK_SIZE - buf_idx;
            let write_len = cmp::min(rem_len, inp.len());

            self.buffer[buf_idx..buf_idx + write_len]
                .copy_from_slice(&inp[..write_len]);
            self.length += write_len;
            if self.length % BLOCK_SIZE == 0 {
                self.process_block();
            }
            inp = &inp[write_len..];
        }
    }

    pub fn finish(mut self) -> [u8; 32] {
        // pad buffer with a single 1-bit then all 0s, until there are exactly 8
        // bytes remaining
        let data_len = self.length as u64;

        let zeroes = [0; BLOCK_SIZE - 8];
        self.input(&[0x80]);
        if self.length % BLOCK_SIZE > zeroes.len() {
            self.input(&zeroes);
        }
        let pad_length = zeroes.len() - (self.length % BLOCK_SIZE);
        self.input(&zeroes[..pad_length]);
        debug_assert_eq!(self.length % BLOCK_SIZE, zeroes.len());

        self.input(&(8 * data_len).to_be_bytes());
        debug_assert_eq!(self.length % BLOCK_SIZE, 0);

        self.midstate()
    }

    // Algorithm copied from libsecp256k1
    fn process_block(&mut self) {
        debug_assert_eq!(self.buffer.len(), BLOCK_SIZE);

        let mut w = [0u32; 16];
        for (w_val, buff_bytes) in w.iter_mut().zip(self.buffer.chunks_exact(4))
        {
            *w_val = u32::from_be_bytes(
                buff_bytes.try_into().expect("4 byte slice"),
            );
        }

        let mut a = self.h[0];
        let mut b = self.h[1];
        let mut c = self.h[2];
        let mut d = self.h[3];
        let mut e = self.h[4];
        let mut f = self.h[5];
        let mut g = self.h[6];
        let mut h = self.h[7];

        round!(a, b, c, d, e, f, g, h, 0x428a2f98, w[0]);
        round!(h, a, b, c, d, e, f, g, 0x71374491, w[1]);
        round!(g, h, a, b, c, d, e, f, 0xb5c0fbcf, w[2]);
        round!(f, g, h, a, b, c, d, e, 0xe9b5dba5, w[3]);
        round!(e, f, g, h, a, b, c, d, 0x3956c25b, w[4]);
        round!(d, e, f, g, h, a, b, c, 0x59f111f1, w[5]);
        round!(c, d, e, f, g, h, a, b, 0x923f82a4, w[6]);
        round!(b, c, d, e, f, g, h, a, 0xab1c5ed5, w[7]);
        round!(a, b, c, d, e, f, g, h, 0xd807aa98, w[8]);
        round!(h, a, b, c, d, e, f, g, 0x12835b01, w[9]);
        round!(g, h, a, b, c, d, e, f, 0x243185be, w[10]);
        round!(f, g, h, a, b, c, d, e, 0x550c7dc3, w[11]);
        round!(e, f, g, h, a, b, c, d, 0x72be5d74, w[12]);
        round!(d, e, f, g, h, a, b, c, 0x80deb1fe, w[13]);
        round!(c, d, e, f, g, h, a, b, 0x9bdc06a7, w[14]);
        round!(b, c, d, e, f, g, h, a, 0xc19bf174, w[15]);

        round!(a, b, c, d, e, f, g, h, 0xe49b69c1, w[0], w[14], w[9], w[1]);
        round!(h, a, b, c, d, e, f, g, 0xefbe4786, w[1], w[15], w[10], w[2]);
        round!(g, h, a, b, c, d, e, f, 0x0fc19dc6, w[2], w[0], w[11], w[3]);
        round!(f, g, h, a, b, c, d, e, 0x240ca1cc, w[3], w[1], w[12], w[4]);
        round!(e, f, g, h, a, b, c, d, 0x2de92c6f, w[4], w[2], w[13], w[5]);
        round!(d, e, f, g, h, a, b, c, 0x4a7484aa, w[5], w[3], w[14], w[6]);
        round!(c, d, e, f, g, h, a, b, 0x5cb0a9dc, w[6], w[4], w[15], w[7]);
        round!(b, c, d, e, f, g, h, a, 0x76f988da, w[7], w[5], w[0], w[8]);
        round!(a, b, c, d, e, f, g, h, 0x983e5152, w[8], w[6], w[1], w[9]);
        round!(h, a, b, c, d, e, f, g, 0xa831c66d, w[9], w[7], w[2], w[10]);
        round!(g, h, a, b, c, d, e, f, 0xb00327c8, w[10], w[8], w[3], w[11]);
        round!(f, g, h, a, b, c, d, e, 0xbf597fc7, w[11], w[9], w[4], w[12]);
        round!(e, f, g, h, a, b, c, d, 0xc6e00bf3, w[12], w[10], w[5], w[13]);
        round!(d, e, f, g, h, a, b, c, 0xd5a79147, w[13], w[11], w[6], w[14]);
        round!(c, d, e, f, g, h, a, b, 0x06ca6351, w[14], w[12], w[7], w[15]);
        round!(b, c, d, e, f, g, h, a, 0x14292967, w[15], w[13], w[8], w[0]);

        round!(a, b, c, d, e, f, g, h, 0x27b70a85, w[0], w[14], w[9], w[1]);
        round!(h, a, b, c, d, e, f, g, 0x2e1b2138, w[1], w[15], w[10], w[2]);
        round!(g, h, a, b, c, d, e, f, 0x4d2c6dfc, w[2], w[0], w[11], w[3]);
        round!(f, g, h, a, b, c, d, e, 0x53380d13, w[3], w[1], w[12], w[4]);
        round!(e, f, g, h, a, b, c, d, 0x650a7354, w[4], w[2], w[13], w[5]);
        round!(d, e, f, g, h, a, b, c, 0x766a0abb, w[5], w[3], w[14], w[6]);
        round!(c, d, e, f, g, h, a, b, 0x81c2c92e, w[6], w[4], w[15], w[7]);
        round!(b, c, d, e, f, g, h, a, 0x92722c85, w[7], w[5], w[0], w[8]);
        round!(a, b, c, d, e, f, g, h, 0xa2bfe8a1, w[8], w[6], w[1], w[9]);
        round!(h, a, b, c, d, e, f, g, 0xa81a664b, w[9], w[7], w[2], w[10]);
        round!(g, h, a, b, c, d, e, f, 0xc24b8b70, w[10], w[8], w[3], w[11]);
        round!(f, g, h, a, b, c, d, e, 0xc76c51a3, w[11], w[9], w[4], w[12]);
        round!(e, f, g, h, a, b, c, d, 0xd192e819, w[12], w[10], w[5], w[13]);
        round!(d, e, f, g, h, a, b, c, 0xd6990624, w[13], w[11], w[6], w[14]);
        round!(c, d, e, f, g, h, a, b, 0xf40e3585, w[14], w[12], w[7], w[15]);
        round!(b, c, d, e, f, g, h, a, 0x106aa070, w[15], w[13], w[8], w[0]);

        round!(a, b, c, d, e, f, g, h, 0x19a4c116, w[0], w[14], w[9], w[1]);
        round!(h, a, b, c, d, e, f, g, 0x1e376c08, w[1], w[15], w[10], w[2]);
        round!(g, h, a, b, c, d, e, f, 0x2748774c, w[2], w[0], w[11], w[3]);
        round!(f, g, h, a, b, c, d, e, 0x34b0bcb5, w[3], w[1], w[12], w[4]);
        round!(e, f, g, h, a, b, c, d, 0x391c0cb3, w[4], w[2], w[13], w[5]);
        round!(d, e, f, g, h, a, b, c, 0x4ed8aa4a, w[5], w[3], w[14], w[6]);
        round!(c, d, e, f, g, h, a, b, 0x5b9cca4f, w[6], w[4], w[15], w[7]);
        round!(b, c, d, e, f, g, h, a, 0x682e6ff3, w[7], w[5], w[0], w[8]);
        round!(a, b, c, d, e, f, g, h, 0x748f82ee, w[8], w[6], w[1], w[9]);
        round!(h, a, b, c, d, e, f, g, 0x78a5636f, w[9], w[7], w[2], w[10]);
        round!(g, h, a, b, c, d, e, f, 0x84c87814, w[10], w[8], w[3], w[11]);
        round!(f, g, h, a, b, c, d, e, 0x8cc70208, w[11], w[9], w[4], w[12]);
        round!(e, f, g, h, a, b, c, d, 0x90befffa, w[12], w[10], w[5], w[13]);
        round!(d, e, f, g, h, a, b, c, 0xa4506ceb, w[13], w[11], w[6], w[14]);
        round!(c, d, e, f, g, h, a, b, 0xbef9a3f7, w[14], w[12], w[7], w[15]);
        round!(b, c, d, e, f, g, h, a, 0xc67178f2, w[15], w[13], w[8], w[0]);

        self.h[0] = self.h[0].wrapping_add(a);
        self.h[1] = self.h[1].wrapping_add(b);
        self.h[2] = self.h[2].wrapping_add(c);
        self.h[3] = self.h[3].wrapping_add(d);
        self.h[4] = self.h[4].wrapping_add(e);
        self.h[5] = self.h[5].wrapping_add(f);
        self.h[6] = self.h[6].wrapping_add(g);
        self.h[7] = self.h[7].wrapping_add(h);
    }
}
