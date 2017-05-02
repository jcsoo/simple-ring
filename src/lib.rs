#![feature(const_fn)]
#![no_std]

use core::cmp;
use core::cell::Cell;

macro_rules! ring_buf {
    ($size:expr, $t:ident) => {                
        RingBuf { reader: Cell::new(0), writer: Cell::new(0), length: $size, buffer: &mut [::core::$t::MIN; $size] as *mut Array<$t>}
    }
}

pub struct RingBuf<T> {
    reader: Cell<usize>,
    writer: Cell<usize>,
    length: usize,
    buffer: *mut Array<T>,
}

impl<T: Copy> RingBuf<T> {
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

    fn phy(&self, index: usize) -> usize {
        index % self.cap()
    }

    fn incr_reader(&self) {
        assert!(!self.is_empty(), "Attempted to increment empty reader");
        self.reader.set(self.reader.get().wrapping_add(1));
    }

    fn incr_writer(&self) {        
        assert!(!self.is_full(), "Attempted to increment full writer");
        self.writer.set(self.writer.get().wrapping_add(1));     
    }

    pub fn enqueue(&self, value: T) -> bool {
        if self.is_full() {
            false
        } else {
            let writer = self.phy(self.writer.get());
            unsafe { (&mut *self.buffer).set(writer, value); }
            self.incr_writer();
            true
        }
    }

    pub fn dequeue(&self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let reader = self.phy(self.reader.get());
            let value = unsafe { (&mut *self.buffer).get(reader) };
            self.incr_reader();
            Some(value)
        }
    }

    pub fn write(&self, buf: &[T]) -> usize {
        let n = cmp::min(self.rem(), buf.len());
        for i in 0..n {
            self.enqueue(buf[i]);
        }
        n
    }

    pub fn read(&self, buf: &mut [T]) -> usize {
        let n = cmp::min(self.len(), buf.len());
        for i in 0..n {
            buf[i] = self.dequeue().expect("Ring buffer is empty");
        }
        n
    }
}

pub trait Array<T> {
    fn get(&mut self, index: usize) -> T;
    fn set(&mut self, index: usize, value: T);
}

macro_rules! impl_array_recursive {
    ($($size:expr),*) => {
        $(
            impl_array!($size, u8);
            impl_array!($size, u16);
            impl_array!($size, u32);
            impl_array!($size, usize);
            impl_array!($size, i8);
            impl_array!($size, i16);
            impl_array!($size, i32);
            impl_array!($size, isize);
        )*
             
    }
}

macro_rules! impl_array {
    ($size:expr, $t:ty) => {
        impl Array<$t> for [$t; $size] {
            fn get(&mut self, index: usize) -> $t {
                self[index]
            }
            fn set(&mut self, index: usize, value: $t) {
                self[index] = value
            }
        }
    }
}

impl_array_recursive!(1, 2, 4, 8, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 768, 1024, 1536, 2048, 3072, 4096, 8192, 16384, 32768, 65536);


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_dequeue() {
        let rbuf = ring_buf!(16, u8);
        
        for i in 0..16 {
            assert_eq!(rbuf.enqueue(i as u8), true);
        }
        assert_eq!(rbuf.enqueue(0), false);
        for i in 0..16 {
            assert_eq!(rbuf.dequeue(), Some(i as u8));
        }
        assert_eq!(rbuf.dequeue(), None);
    }

    #[test]
    fn test_enqueue_dequeue_u32() {
        let rbuf = ring_buf!(16, u32);
        
        for i in 0..16 {
            assert_eq!(rbuf.enqueue(i as u32), true);
        }
        assert_eq!(rbuf.enqueue(0), false);
        for i in 0..16 {
            assert_eq!(rbuf.dequeue(), Some(i as u32));
        }
        assert_eq!(rbuf.dequeue(), None);
    }    

    #[test]
    fn test_write_read() {
        let rbuf = ring_buf!(16, u8);

        let src: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let mut dst = [0u8; 16];

        rbuf.write(&src);
        let n = rbuf.read(&mut dst);
        assert_eq!(n, 16);
        for i in 0..16 {
            assert_eq!(src[i], dst[i]);
        }
    }

    struct Driver<'a> {
        rbuf: &'a RingBuf<u8>,
    }

    impl<'a> Driver<'a> {
        pub fn run(&mut self) {
            self.rbuf.write(b"ABC");
        }
    }

    #[test]
    fn test_driver() {
        let rbuf = ring_buf!(16, u8);
        {
            let mut d = Driver { rbuf: &rbuf };
            d.run();

            let mut dst = [0u8; 16];
            let n = rbuf.read(&mut dst);
            assert_eq!(&dst[..n], b"ABC");
        }
    }

    #[test]
    fn test_static_driver() {
        static mut RBUF: RingBuf<u8> = ring_buf!(16, u8);
        static mut DRV: Option<Driver> = None;
        {            
            unsafe {
                DRV = Some(Driver { rbuf: &RBUF });
                &DRV.as_mut().unwrap().run();
            }

        }
        unsafe {
            let mut dst = [0u8; 16];
            let n = RBUF.read(&mut dst);
            assert_eq!(&dst[..n], b"ABC");
        }

    }
}
