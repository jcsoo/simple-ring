#![allow(dead_code)]
#![feature(const_fn)]
//#![no_std]
extern crate core;

use core::cmp;
//use core::mem;
use core::marker::PhantomData;
use core::cell::Cell;

macro_rules! impl_byte_array_recursive {
    ($($size:expr),*) => {
        $(
            impl_byte_array!($size);
        )*
             
    }
}

macro_rules! impl_byte_array {
    ($size:expr) => {
        impl ByteArray for [u8; $size] {
            fn get(&mut self, index: usize) -> u8 {
                self[index]
            }
            fn set(&mut self, index: usize, value: u8) {
                self[index] = value
            }
        }
    }
}

impl_byte_array_recursive!(1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096);

macro_rules! static_ring_buf {
    ($size:expr) => {
        {
            static mut RBUF: RingBuf = RingBuf { reader: Cell::new(0), writer: Cell::new(0), length: $size, buffer: &mut [0u8; $size] as *mut ByteArray};
            unsafe { (RBUF.reader(), RBUF.writer() )}
        }
    }
}

macro_rules! ring_buf {
    ($size:expr) => {
        {
            let mut rbuf = RingBuf { reader: Cell::new(0), writer: Cell::new(0), length: $size, buffer: &mut [0u8; $size] as *mut ByteArray};
            (rbuf.reader(), rbuf.writer())
        }
    }
}

pub trait ByteArray {
    fn get(&mut self, index: usize) -> u8;
    fn set(&mut self, index: usize, value: u8);
}


pub struct RingBuf {
    reader: Cell<usize>,
    writer: Cell<usize>,
    length: usize,
    buffer: *mut ByteArray,
}

impl RingBuf {
    pub fn reader<'a>(&'a mut self) -> RingReader<RingBuf> {
        RingReader { ring: self, _phantom: PhantomData }
    }

    pub fn writer(&mut self) -> RingWriter<RingBuf> {
        RingWriter { ring: self, _phantom: PhantomData }
    }    

    fn cap(&self) -> usize {
        self.length
    }

    fn len(&self) -> usize {
        self.writer.get().wrapping_sub(self.reader.get())
    }

    fn rem(&self) -> usize {
        self.cap().wrapping_sub(self.len())
    }

    fn is_empty(&self) -> bool {
        self.reader.get() == self.writer.get()
    }

    fn is_full(&self) -> bool {
        self.len() == self.cap()
    }

    fn incr_reader(&self) {
        assert!(!self.is_empty(), "Attempted to increment empty reader");
        self.reader.set(self.reader.get().wrapping_add(1));
    }

    fn incr_writer(&self) {        
        assert!(!self.is_full(), "Attempted to increment full writer");
        self.writer.set(self.writer.get().wrapping_add(1));     
    }

    fn phy(&self, index: usize) -> usize {
        index % self.cap()
    }

    fn enqueue(&self, value: u8) -> bool {
        if self.is_full() {
            false
        } else {
            let writer = self.phy(self.writer.get());
            unsafe { (&mut *self.buffer).set(writer, value); }
            self.incr_writer();
            true
        }
    }

    fn dequeue(&self) -> Option<u8> {
        if self.is_empty() {
            None
        } else {
            let reader = self.phy(self.reader.get());
            let value = unsafe { (&mut *self.buffer).get(reader) };
            self.incr_reader();
            Some(value)
        }
    }

    fn write(&self, buf: &[u8]) -> usize {
        let n = cmp::min(self.rem(), buf.len());
        for i in 0..n {
            self.enqueue(buf[i]);
        }
        n
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        let n = cmp::min(self.len(), buf.len());
        for i in 0..n {
            buf[i] = self.dequeue().expect("Ring buffer is empty");
        }
        n
    }

}

pub struct RingReader<T> {
    ring: *mut RingBuf,
    _phantom: PhantomData<T>
}

impl<T> RingReader<T> {
    pub fn dequeue(&mut self) -> Option<u8> {
        let ring = unsafe { &mut *self.ring};
        ring.dequeue()
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let ring = unsafe { &mut *self.ring};
        ring.read(buf)
    }
}

pub struct RingWriter<T> {
    ring: *mut RingBuf,
    _phantom: PhantomData<T>
}

impl<T> RingWriter<T> {
    pub fn enqueue(&mut self, value: u8) -> bool {
        let ring = unsafe { &mut *self.ring};
        ring.enqueue(value)
    }
    pub fn write(&mut self, buf: &[u8]) -> usize {
        let ring = unsafe { &mut *self.ring};
        ring.write(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytearray() {
        let mut arr = [0u8; 16];
        arr.set(0, 1);
    }

    #[test]
    fn test_enqueue_dequeue() {
        let (mut reader, mut writer) = static_ring_buf!(16);
        
        for i in 0..16 {
            assert_eq!(writer.enqueue(i as u8), true);
        }
        for i in 0..16 {
            assert_eq!(reader.dequeue(), Some(i as u8));
        }
    }
    #[test]
    fn test_write_read() {
        let (mut reader, mut writer) = static_ring_buf!(16);

        let src: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let mut dst = [0u8; 16];

        writer.write(&src);
        let n = reader.read(&mut dst);
        assert_eq!(n, 16);
        for i in 0..16 {
            assert_eq!(src[i], dst[i]);
        }
    }

    #[test]
    fn test_driver() {
        pub struct Driver {
            writer: RingWriter<RingBuf>
        }

        impl Driver {
            pub fn run(&mut self) {
                self.writer.write(b"ABC");
            }
        }

        let (mut reader, writer) = ring_buf!(16);
        let mut d = Driver { writer: writer };
        d.run();

        let mut dst = [0u8; 16];
        let n = reader.read(&mut dst);
        assert_eq!(&dst[..n], b"ABC");
    }

    #[test]
    fn test_static_driver() {
        pub struct Driver {
            writer: RingWriter<RingBuf>
        }

        impl Driver {
            pub fn run(&mut self) {
                self.writer.write(b"ABC");
            }
        }
        static mut DRV: Option<Driver> = None;
        let (mut reader, writer) = ring_buf!(16);
        {            
            unsafe {
                DRV = Some(Driver { writer: writer });
                &DRV.as_mut().unwrap().run();
            }

        }
        unsafe {
            let d = &DRV;
        }
        let mut dst = [0u8; 16];
        let n = reader.read(&mut dst);
        assert_eq!(&dst[..n], b"ABC");
    }    

    #[test]
    fn test_static() {
    }
}
