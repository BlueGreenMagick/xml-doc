use encoding_rs::Decoder;
use std::io::{BufRead, Read};


pub(crate) struct DecodeReader<R: Read> {
    decoder: Option<Decoder>,
    inner: R,
    undecoded: [u8; 4096],
    undecoded_pos: usize,
    undecoded_cap: usize,
    remaining: [u8; 32], // Is there an encoding with > 32 bytes for a char?
    decoded: [u8; 12288],
    decoded_pos: usize,
    decoded_cap: usize,
    done: bool,
}

impl<R: Read> DecodeReader<R> {
    // If Decoder is not set, don't decode.
    pub(crate) fn new(reader: R, decoder: Option<Decoder>) -> DecodeReader<R> {
        DecodeReader {
            decoder,
            inner: reader,
            undecoded: [0; 4096],
            undecoded_pos: 0,
            undecoded_cap: 0,
            remaining: [0; 32],
            decoded: [0; 12288],
            decoded_pos: 0,
            decoded_cap: 0,
            done: false,
        }
    }

    pub(crate) fn set_decoder(&mut self, dec: Option<Decoder>) {
        self.decoder = dec;
        self.done = false;
    }

    // Call this only when decoder is Some
    fn fill_buf_decode(&mut self) -> std::io::Result<&[u8]> {
        if self.decoded_pos >= self.decoded_cap {
            debug_assert!(self.decoded_pos == self.decoded_cap);
            if self.done {
                return Ok(&[]);
            }
            let remaining = self.undecoded_cap - self.undecoded_pos;
            if remaining <= 32 {
                // Move remaining undecoded bytes at the end to start
                self.remaining[..remaining]
                    .copy_from_slice(&self.undecoded[self.undecoded_pos..self.undecoded_cap]);
                self.undecoded[..remaining].copy_from_slice(&self.remaining[..remaining]);
                // Fill undecoded buffer
                let read = self.inner.read(&mut self.undecoded[remaining..])?;
                self.done = read == 0;
                self.undecoded_pos = 0;
                self.undecoded_cap = remaining + read;
            }

            // Fill decoded buffer
            let (_res, read, written, _replaced) = self.decoder.as_mut().unwrap().decode_to_utf8(
                &self.undecoded[self.undecoded_pos..self.undecoded_cap],
                &mut self.decoded,
                self.done,
            );
            self.undecoded_pos += read;
            self.decoded_cap = written;
            self.decoded_pos = 0;
        }
        Ok(&self.decoded[self.decoded_pos..self.decoded_cap])
    }

    fn fill_buf_without_decode(&mut self) -> std::io::Result<&[u8]> {
        if self.undecoded_pos >= self.undecoded_cap {
            debug_assert!(self.undecoded_pos == self.undecoded_cap);
            self.undecoded_cap = self.inner.read(&mut self.undecoded)?;
            self.undecoded_pos = 0;
        }
        Ok(&self.undecoded[self.undecoded_pos..self.undecoded_cap])
    }
}

impl<R: Read> Read for DecodeReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        (&self.decoded[..]).read(buf)
    }
}

impl<R: Read> BufRead for DecodeReader<R> {
    // Decoder may change from None to Some.
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        match &self.decoder {
            Some(_) => self.fill_buf_decode(),
            None => self.fill_buf_without_decode(),
        }
    }
    fn consume(&mut self, amt: usize) {
        match &self.decoder {
            Some(_) => {
                self.decoded_pos = std::cmp::min(self.decoded_pos + amt, self.decoded_cap);
            }
            None => {
                self.undecoded_pos = std::cmp::min(self.undecoded_pos + amt, self.undecoded_cap);
            }
        }
    }
}